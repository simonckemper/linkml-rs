//! Example demonstrating Mermaid diagram generation from LinkML schemas
//!
//! Mermaid is a JavaScript-based diagramming and charting tool that renders
//! Markdown-inspired text definitions to create diagrams dynamically.

use linkml_core::prelude::*;
use linkml_service::generator::{
    Generator, GeneratorOptions, MermaidDiagramType, MermaidGenerator,
};

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Create a sample schema for a library management system
    let mut schema = SchemaDefinition::default();
    schema.name = Some("LibrarySchema".to_string());
    schema.id = Some("https://example.org/library".to_string());
    schema.description = Some("A schema for library management".to_string());

    // Base class
    let mut identifiable = ClassDefinition::default();
    identifiable.abstract_ = Some(true);
    identifiable.description = Some("Base class for all identifiable entities".to_string());
    identifiable.slots = vec!["id".to_string(), "created_at".to_string()];
    schema
        .classes
        .insert("Identifiable".to_string(), identifiable);

    // Book class
    let mut book = ClassDefinition::default();
    book.description = Some("A book in the library".to_string());
    book.is_a = Some("Identifiable".to_string());
    book.slots = vec![
        "title".to_string(),
        "isbn".to_string(),
        "authors".to_string(),
        "publisher".to_string(),
        "status".to_string(),
    ];
    schema.classes.insert("Book".to_string(), book);

    // Author class
    let mut author = ClassDefinition::default();
    author.description = Some("An author of books".to_string());
    author.is_a = Some("Identifiable".to_string());
    author.slots = vec![
        "name".to_string(),
        "biography".to_string(),
        "books".to_string(),
    ];
    schema.classes.insert("Author".to_string(), author);

    // Member class
    let mut member = ClassDefinition::default();
    member.description = Some("A library member".to_string());
    member.is_a = Some("Identifiable".to_string());
    member.slots = vec![
        "name".to_string(),
        "email".to_string(),
        "membership_type".to_string(),
        "loans".to_string(),
    ];
    schema.classes.insert("Member".to_string(), member);

    // Loan class
    let mut loan = ClassDefinition::default();
    loan.description = Some("A book loan record".to_string());
    loan.is_a = Some("Identifiable".to_string());
    loan.slots = vec![
        "book".to_string(),
        "member".to_string(),
        "loan_date".to_string(),
        "due_date".to_string(),
        "return_date".to_string(),
    ];
    schema.classes.insert("Loan".to_string(), loan);

    // Publisher class
    let mut publisher = ClassDefinition::default();
    publisher.description = Some("A book publisher".to_string());
    publisher.slots = vec!["name".to_string(), "address".to_string()];
    schema.classes.insert("Publisher".to_string(), publisher);

    // Define slots
    let mut id_slot = SlotDefinition::default();
    id_slot.identifier = Some(true);
    id_slot.range = Some("string".to_string());
    id_slot.required = Some(true);
    schema.slots.insert("id".to_string(), id_slot);

    let mut created_at_slot = SlotDefinition::default();
    created_at_slot.range = Some("datetime".to_string());
    schema
        .slots
        .insert("created_at".to_string(), created_at_slot);

    let mut title_slot = SlotDefinition::default();
    title_slot.range = Some("string".to_string());
    title_slot.required = Some(true);
    schema.slots.insert("title".to_string(), title_slot);

    let mut isbn_slot = SlotDefinition::default();
    isbn_slot.range = Some("string".to_string());
    isbn_slot.pattern = Some(r"^\d{3}-\d{10}$".to_string());
    schema.slots.insert("isbn".to_string(), isbn_slot);

    let mut name_slot = SlotDefinition::default();
    name_slot.range = Some("string".to_string());
    name_slot.required = Some(true);
    schema.slots.insert("name".to_string(), name_slot);

    let mut email_slot = SlotDefinition::default();
    email_slot.range = Some("string".to_string());
    email_slot.pattern = Some(r"^\S+@\S+\.\S+$".to_string());
    schema.slots.insert("email".to_string(), email_slot);

    let mut authors_slot = SlotDefinition::default();
    authors_slot.range = Some("Author".to_string());
    authors_slot.multivalued = Some(true);
    schema.slots.insert("authors".to_string(), authors_slot);

    let mut books_slot = SlotDefinition::default();
    books_slot.range = Some("Book".to_string());
    books_slot.multivalued = Some(true);
    schema.slots.insert("books".to_string(), books_slot);

    let mut publisher_slot = SlotDefinition::default();
    publisher_slot.range = Some("Publisher".to_string());
    schema.slots.insert("publisher".to_string(), publisher_slot);

    let mut book_slot = SlotDefinition::default();
    book_slot.range = Some("Book".to_string());
    book_slot.required = Some(true);
    schema.slots.insert("book".to_string(), book_slot);

    let mut member_slot = SlotDefinition::default();
    member_slot.range = Some("Member".to_string());
    member_slot.required = Some(true);
    schema.slots.insert("member".to_string(), member_slot);

    let mut loans_slot = SlotDefinition::default();
    loans_slot.range = Some("Loan".to_string());
    loans_slot.multivalued = Some(true);
    schema.slots.insert("loans".to_string(), loans_slot);

    let mut date_slots = vec!["loan_date", "due_date", "return_date"];
    for slot_name in date_slots {
        let mut slot = SlotDefinition::default();
        slot.range = Some("date".to_string());
        if slot_name != "return_date" {
            slot.required = Some(true);
        }
        schema.slots.insert(slot_name.to_string(), slot);
    }

    let mut string_slots = vec!["biography", "address"];
    for slot_name in string_slots {
        let mut slot = SlotDefinition::default();
        slot.range = Some("string".to_string());
        schema.slots.insert(slot_name.to_string(), slot);
    }

    let mut status_slot = SlotDefinition::default();
    status_slot.range = Some("BookStatus".to_string());
    schema.slots.insert("status".to_string(), status_slot);

    let mut membership_slot = SlotDefinition::default();
    membership_slot.range = Some("MembershipType".to_string());
    schema
        .slots
        .insert("membership_type".to_string(), membership_slot);

    // Define enumerations
    let mut book_status = EnumDefinition::default();
    book_status.description = Some("Status of a book".to_string());
    book_status.permissible_values = vec![
        PermissibleValue::Simple("AVAILABLE".to_string()),
        PermissibleValue::Simple("LOANED".to_string()),
        PermissibleValue::Simple("RESERVED".to_string()),
        PermissibleValue::Simple("MAINTENANCE".to_string()),
        PermissibleValue::Simple("LOST".to_string()),
    ];
    schema.enums.insert("BookStatus".to_string(), book_status);

    let mut membership_type = EnumDefinition::default();
    membership_type.description = Some("Type of library membership".to_string());
    membership_type.permissible_values = vec![
        PermissibleValue::Simple("BASIC".to_string()),
        PermissibleValue::Simple("PREMIUM".to_string()),
        PermissibleValue::Simple("STUDENT".to_string()),
        PermissibleValue::Simple("SENIOR".to_string()),
    ];
    schema
        .enums
        .insert("MembershipType".to_string(), membership_type);

    println!("=== Mermaid Diagram Generation Examples ===
");

    // Example 1: Entity Relationship Diagram
    println!("1. Entity Relationship Diagram:");
    println!("-------------------------------");
    let er_generator = MermaidGenerator::new();
    let er_content = er_generator.generate_with_options(&schema, &GeneratorOptions::default())?;
    println!("Generated: library_er.mermaid");
    std::fs::write("library_er.mermaid", &er_content)?;
    println!("Saved to: library_er.mermaid");
    println!(
        "
Preview:
{}
",
        er_content.lines().take(20).collect::<Vec<_>>().join("
")
    );

    // Example 2: Class Diagram
    println!("2. Class Diagram:");
    println!("-----------------");
    let class_generator =
        MermaidGenerator::new().with_diagram_type(MermaidDiagramType::ClassDiagram);
    let class_content =
        class_generator.generate_with_options(&schema, &GeneratorOptions::default())?;
    std::fs::write("library_class.mermaid", &class_content)?;
    println!("Generated: {}", result[0].filename);
    println!("Saved to: library_class.mermaid
");

    // Example 3: State Diagram
    println!("3. State Diagram (Book Status):");
    println!("-------------------------------");
    let state_generator =
        MermaidGenerator::new().with_diagram_type(MermaidDiagramType::StateDiagram);
    let state_content =
        state_generator.generate_with_options(&schema, &GeneratorOptions::default())?;
    std::fs::write("library_states.mermaid", &state_content)?;
    println!("Generated: {}", result[0].filename);
    println!("Saved to: library_states.mermaid
");

    // Example 4: Flowchart
    println!("4. Schema Structure Flowchart:");
    println!("------------------------------");
    let flow_generator = MermaidGenerator::new().with_diagram_type(MermaidDiagramType::Flowchart);
    let flow_content =
        flow_generator.generate_with_options(&schema, &GeneratorOptions::default())?;
    std::fs::write("library_flow.mermaid", &flow_content)?;
    println!("Generated: {}", result[0].filename);
    println!("Saved to: library_flow.mermaid
");

    // Create an HTML file to preview all diagrams
    let html_preview = r#"<!DOCTYPE html>
<html>
<head>
    <title>Library Schema Diagrams</title>
    <script type="module">
        import mermaid from 'https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.esm.min.mjs';
        mermaid.initialize({ startOnLoad: true });
    </script>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        h2 { color: #333; }
        .diagram { margin: 20px 0; padding: 20px; border: 1px solid #ddd; background: #f9f9f9; }
    </style>
</head>
<body>
    <h1>Library Management Schema Visualizations</h1>

    <div class="diagram">
        <h2>Entity Relationship Diagram</h2>
        <pre class="mermaid">
"#;

    let mut html = html_preview.to_string();
    html.push_str(&std::fs::read_to_string("library_er.mermaid")?);
    html.push_str("
        </pre>
    </div>
    
    <div class=\"diagram\">
        <h2>Class Diagram</h2>
        <pre class=\"mermaid\">
");
    html.push_str(&std::fs::read_to_string("library_class.mermaid")?);
    html.push_str("
        </pre>
    </div>
    
    <div class=\"diagram\">
        <h2>State Diagram</h2>
        <pre class=\"mermaid\">
");
    html.push_str(&std::fs::read_to_string("library_states.mermaid")?);
    html.push_str("
        </pre>
    </div>
    
    <div class=\"diagram\">
        <h2>Schema Structure Flowchart</h2>
        <pre class=\"mermaid\">
");
    html.push_str(&std::fs::read_to_string("library_flow.mermaid")?);
    html.push_str("
        </pre>
    </div>
</body>
</html>");

    std::fs::write("library_diagrams.html", html)?;

    println!("✅ Mermaid diagram generation complete!");
    println!("
To view the diagrams:");
    println!("1. Open library_diagrams.html in a web browser");
    println!("2. Or use the Mermaid Live Editor: https://mermaid.live/");
    println!("3. Or install Mermaid CLI: npm install -g @mermaid-js/mermaid-cli");
    println!("   Then: mmdc -i library_er.mermaid -o library_er.png");
    println!("
Mermaid diagrams can be embedded in:");
    println!("- GitHub README files");
    println!("- Markdown documentation");
    println!("- Web applications");
    println!("- Jupyter notebooks");

    Ok(())
}
