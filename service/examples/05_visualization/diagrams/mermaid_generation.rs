//! Generate Mermaid diagrams from a LinkML schema.
//!
//! The example builds a small library schema in memory, renders two different
//! Mermaid diagram types, and saves them alongside an HTML preview so you can
//! open the visualisation directly in a browser.

use anyhow::{Context, Result};
use linkml_core::prelude::*;
use linkml_service::generator::{MermaidDiagramType, MermaidGenerator};
use std::fs;
use std::path::Path;

const OUTPUT_DIR: &str = "/tmp";

fn main() -> Result<()> {
    let schema = build_sample_schema();

    let er_generator = MermaidGenerator::new();
    let class_generator =
        MermaidGenerator::new().with_diagram_type(MermaidDiagramType::ClassDiagram);

    let er_diagram = er_generator
        .generate(&schema)
        .context("generating ER diagram")?;
    let class_diagram = class_generator
        .generate(&schema)
        .context("generating class diagram")?;

    let er_path = Path::new(OUTPUT_DIR).join("library_er.mermaid");
    let class_path = Path::new(OUTPUT_DIR).join("library_classes.mermaid");
    fs::write(&er_path, er_diagram).with_context(|| format!("writing {}", er_path.display()))?;
    fs::write(&class_path, class_diagram)
        .with_context(|| format!("writing {}", class_path.display()))?;

    let html_path = Path::new(OUTPUT_DIR).join("library_mermaid_preview.html");
    fs::write(&html_path, render_html_preview(&er_path, &class_path))
        .with_context(|| format!("writing {}", html_path.display()))?;

    println!("Mermaid diagrams written to {}", OUTPUT_DIR);
    println!("  • {}", er_path.display());
    println!("  • {}", class_path.display());
    println!("  • {}", html_path.display());
    println!("Open the HTML file in a browser to view the diagrams.");

    Ok(())
}

fn build_sample_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::default();
    schema.id = "https://example.org/library".to_string();
    schema.name = "LibrarySchema".to_string();
    schema.description = Some("A small library domain model".to_string());

    let mut identifiable = ClassDefinition::default();
    identifiable.name = Some("Identifiable".to_string());
    identifiable.abstract_ = Some(true);
    identifiable.slots = vec!["id".to_string(), "created_at".to_string()];
    schema
        .classes
        .insert("Identifiable".to_string(), identifiable);

    let mut book = ClassDefinition::default();
    book.name = Some("Book".to_string());
    book.description = Some("A book in the catalogue".to_string());
    book.is_a = Some("Identifiable".to_string());
    book.slots = vec![
        "title".to_string(),
        "isbn".to_string(),
        "authors".to_string(),
        "publisher".to_string(),
        "status".to_string(),
    ];
    schema.classes.insert("Book".to_string(), book);

    let mut person = ClassDefinition::default();
    person.name = Some("Person".to_string());
    person.description = Some("A library member".to_string());
    person.is_a = Some("Identifiable".to_string());
    person.slots = vec!["name".to_string(), "email".to_string()];
    schema.classes.insert("Person".to_string(), person);

    let mut loan = ClassDefinition::default();
    loan.name = Some("Loan".to_string());
    loan.description = Some("Tracks who borrowed a book".to_string());
    loan.slots = vec![
        "book".to_string(),
        "borrower".to_string(),
        "loan_date".to_string(),
        "due_date".to_string(),
    ];
    schema.classes.insert("Loan".to_string(), loan);

    let mut status_enum = EnumDefinition::default();
    status_enum.description = Some("Availability status".to_string());
    status_enum.permissible_values = vec![
        PermissibleValue::Simple("AVAILABLE".to_string()),
        PermissibleValue::Simple("CHECKED_OUT".to_string()),
        PermissibleValue::Simple("LOST".to_string()),
    ];
    schema.enums.insert("BookStatus".to_string(), status_enum);

    schema
        .slots
        .insert("id".to_string(), SlotDefinition::default());
    schema
        .slots
        .insert("created_at".to_string(), SlotDefinition::default());
    schema
        .slots
        .insert("title".to_string(), SlotDefinition::default());
    schema
        .slots
        .insert("isbn".to_string(), SlotDefinition::default());
    schema
        .slots
        .insert("authors".to_string(), SlotDefinition::default());
    schema
        .slots
        .insert("publisher".to_string(), SlotDefinition::default());
    schema
        .slots
        .insert("status".to_string(), SlotDefinition::default());
    schema
        .slots
        .insert("name".to_string(), SlotDefinition::default());
    schema
        .slots
        .insert("email".to_string(), SlotDefinition::default());
    schema
        .slots
        .insert("book".to_string(), SlotDefinition::default());
    schema
        .slots
        .insert("borrower".to_string(), SlotDefinition::default());
    schema
        .slots
        .insert("loan_date".to_string(), SlotDefinition::default());
    schema
        .slots
        .insert("due_date".to_string(), SlotDefinition::default());

    schema
}

fn render_html_preview(er_path: &Path, class_path: &Path) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Library Mermaid Diagrams</title>
  <script type="module">
    import mermaid from 'https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.esm.min.mjs';
    mermaid.initialize({{ startOnLoad: true, securityLevel: 'strict' }});
  </script>
  <style>
    body {{ font-family: Arial, sans-serif; margin: 2rem; background: #f6f6f6; }}
    h2 {{ margin-top: 2rem; }}
    pre {{ background: #fff; border: 1px solid #ddd; padding: 1rem; overflow-x: auto; }}
  </style>
</head>
<body>
  <h1>Library Schema Visualisation</h1>
  <p>Generated by the LinkML Mermaid example.</p>

  <h2>Entity Relationship Diagram</h2>
  <pre class="mermaid">
{er}
  </pre>

  <h2>Class Diagram</h2>
  <pre class="mermaid">
{class_diagram}
  </pre>
</body>
</html>
"#,
        er = fs::read_to_string(er_path).unwrap_or_default(),
        class_diagram = fs::read_to_string(class_path).unwrap_or_default()
    )
}
