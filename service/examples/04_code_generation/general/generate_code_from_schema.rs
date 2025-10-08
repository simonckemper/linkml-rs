//! Generate code for a LinkML schema located on disk.
//!
//! This example loads the ISO3166 country schema and emits a few different
//! language targets. The output is written to `/tmp` so you can inspect or
//! compile it after running the example.

use anyhow::{Context, Result};
use linkml_core::prelude::SchemaDefinition;
use linkml_service::generator::{PythonDataclassGenerator, RustGenerator, traits::Generator};
use serde_yaml;
use std::fs;
use std::path::{Path, PathBuf};

const OUTPUT_DIR: &str = "/tmp";

#[tokio::main]
async fn main() -> Result<()> {
    let schema_path = locate_schema("place/polity/country/schema.yaml")?;
    println!("Loading schema from {}", schema_path.display());

    let schema = load_schema(&schema_path)?;
    println!(
        "Loaded schema '{}' with {} classes and {} slots\n",
        schema.name,
        schema.classes.len(),
        schema.slots.len()
    );

    generate_rust(&schema)?;
    generate_python_dataclasses(&schema)?;

    println!("Done. Generated files have been written to {OUTPUT_DIR}.");
    Ok(())
}

fn locate_schema(relative: &str) -> Result<PathBuf> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let schema_path = repo_root.join("examples").join("schema").join(relative);

    if schema_path.exists() {
        Ok(schema_path)
    } else {
        // Fall back to workspace domain schema if available
        let workspace_root = repo_root
            .ancestors()
            .nth(4)
            .ok_or_else(|| anyhow::anyhow!("unable to locate workspace root"))?;
        let fallback = workspace_root.join("domain/schema").join(relative);
        if fallback.exists() {
            Ok(fallback)
        } else {
            Err(anyhow::anyhow!(
                "could not find schema at {} or {}",
                schema_path.display(),
                fallback.display()
            ))
        }
    }
}

fn load_schema(path: &Path) -> Result<SchemaDefinition> {
    let data = fs::read_to_string(path)
        .with_context(|| format!("reading schema file {}", path.display()))?;
    let schema: SchemaDefinition = serde_yaml::from_str(&data).context("parsing schema as YAML")?;
    Ok(schema)
}

fn generate_rust(schema: &SchemaDefinition) -> Result<()> {
    let generator = RustGenerator::new();
    generator
        .validate_schema(schema)
        .context("validating schema for Rust generator")?;
    let code = generator
        .generate(schema)
        .context("running Rust generator")?;

    let output = Path::new(OUTPUT_DIR).join("iso3166_generated.rs");
    fs::write(&output, code).with_context(|| format!("writing {}", output.display()))?;
    println!("• Rust structs -> {}", output.display());
    Ok(())
}

fn generate_python_dataclasses(schema: &SchemaDefinition) -> Result<()> {
    let generator = PythonDataclassGenerator::new();
    generator
        .validate_schema(schema)
        .context("validating schema for Python generator")?;
    let code = generator
        .generate(schema)
        .context("running Python dataclass generator")?;

    let output = Path::new(OUTPUT_DIR).join("iso3166_generated.py");
    fs::write(&output, code).with_context(|| format!("writing {}", output.display()))?;
    println!("• Python dataclasses -> {}", output.display());
    Ok(())
}
