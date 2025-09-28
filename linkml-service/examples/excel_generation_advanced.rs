//! Advanced Excel Generation Example
//!
//! This example demonstrates the advanced features of LinkML's Excel generator,
//! including conditional formatting, data validation, charts, and multi-sheet workbooks.

use linkml_service::{
    Result,
    generator::{ExcelFormat, ExcelGenerator, ExcelOptions},
    schema::{ClassBuilder, Schema, SchemaBuilder, SlotBuilder},
};
use std::path::Path;

fn main() -> Result<()> {
    println!("=== Advanced Excel Generation with LinkML ===
");

    // Create a comprehensive schema for a research project
    let schema = create_research_schema()?;

    // Configure advanced Excel options
    let options = ExcelOptions {
        // Multi-sheet generation
        separate_sheets: true,
        sheet_per_class: true,

        // Formatting options
        use_conditional_formatting: true,
        highlight_required_fields: true,
        color_code_by_type: true,

        // Data validation
        add_dropdown_validation: true,
        add_range_validation: true,
        add_pattern_validation: true,

        // Rich features
        include_charts: true,
        add_pivot_tables: true,
        create_dashboard_sheet: true,

        // Documentation
        add_schema_documentation: true,
        include_examples: true,
        add_tooltips: true,

        // Styling
        use_table_styles: true,
        freeze_headers: true,
        auto_column_width: true,

        // Advanced features
        format: ExcelFormat::Xlsx,
        include_vba_macros: false,
        password_protect: None,
    };

    // Generate Excel workbook
    println!("Generating advanced Excel workbook...");
    let generator = ExcelGenerator::new();
    generator.generate_file(&schema, "research_project.xlsx", Some(options))?;

    println!("✅ Generated: research_project.xlsx");

    // Generate with different configurations
    generate_variants(&schema)?;

    // Show generated features
    println!("
📊 Generated Excel Features:");
    println!("  - Multiple sheets (one per class)");
    println!("  - Conditional formatting for data types");
    println!("  - Data validation dropdowns for enums");
    println!("  - Range validation for numeric fields");
    println!("  - Pattern validation for string fields");
    println!("  - Color-coded required fields");
    println!("  - Auto-generated charts");
    println!("  - Pivot table for analysis");
    println!("  - Dashboard summary sheet");
    println!("  - Rich tooltips with documentation");
    println!("  - Frozen headers for scrolling");
    println!("  - Auto-sized columns");

    Ok(())
}

fn create_research_schema() -> Result<Schema> {
    let mut builder = SchemaBuilder::new("ResearchProject");

    builder
        .id("https://example.com/research-project")
        .description("Schema for managing research projects and publications")
        .version("2.0.0");

    // Add enums for validation
    builder
        .add_enum("ProjectStatus")
        .add_value("planning", "Project is in planning phase")
        .add_value("active", "Project is actively being worked on")
        .add_value("completed", "Project has been completed")
        .add_value("on_hold", "Project is temporarily on hold");

    builder
        .add_enum("PublicationType")
        .add_value("journal", "Peer-reviewed journal article")
        .add_value("conference", "Conference paper")
        .add_value("book", "Book or book chapter")
        .add_value("preprint", "Preprint article")
        .add_value("thesis", "Thesis or dissertation");

    // Research Project class
    builder
        .add_class("ResearchProject")
        .description("A research project with team members and outputs")
        .add_slot("project_id")
        .range("string")
        .required(true)
        .identifier(true)
        .pattern("^PROJ-\\d{4}-\\d{3}$")
        .description("Unique project identifier (format: PROJ-YYYY-NNN)")
        .add_slot("title")
        .range("string")
        .required(true)
        .min_length(10)
        .max_length(200)
        .description("Project title")
        .add_slot("status")
        .range("ProjectStatus")
        .required(true)
        .description("Current project status")
        .add_slot("start_date")
        .range("date")
        .required(true)
        .description("Project start date")
        .add_slot("end_date")
        .range("date")
        .description("Project end date (if completed)")
        .add_slot("budget")
        .range("decimal")
        .minimum_value(0.0)
        .maximum_value(10000000.0)
        .unit("USD")
        .description("Total project budget in USD")
        .add_slot("completion_percentage")
        .range("integer")
        .minimum_value(0)
        .maximum_value(100)
        .unit("percent")
        .description("Project completion percentage");

    // Team Member class
    builder
        .add_class("TeamMember")
        .description("A member of a research team")
        .add_slot("member_id")
        .range("string")
        .required(true)
        .identifier(true)
        .pattern("^TM-\\d{6}$")
        .add_slot("name")
        .range("string")
        .required(true)
        .add_slot("email")
        .range("string")
        .required(true)
        .pattern("^[\\w\\.-]+@[\\w\\.-]+\\.\\w+$")
        .add_slot("role")
        .range("string")
        .required(true)
        .enum_values(vec![
            "PI",
            "Co-PI",
            "Postdoc",
            "PhD Student",
            "Research Assistant",
        ])
        .add_slot("hours_per_week")
        .range("decimal")
        .minimum_value(0.0)
        .maximum_value(40.0)
        .add_slot("project_id")
        .range("string")
        .required(true)
        .description("Associated project ID");

    // Publication class
    builder
        .add_class("Publication")
        .description("A research publication output")
        .add_slot("pub_id")
        .range("string")
        .required(true)
        .identifier(true)
        .pattern("^PUB-\\d{4}-\\d{4}$")
        .add_slot("title")
        .range("string")
        .required(true)
        .add_slot("authors")
        .range("string")
        .required(true)
        .multivalued(true)
        .description("List of author names")
        .add_slot("publication_type")
        .range("PublicationType")
        .required(true)
        .add_slot("year")
        .range("integer")
        .required(true)
        .minimum_value(2020)
        .maximum_value(2030)
        .add_slot("doi")
        .range("string")
        .pattern("^10\\.\\d{4,}/[-._;()/:a-zA-Z0-9]+$")
        .description("Digital Object Identifier")
        .add_slot("project_id")
        .range("string")
        .required(true)
        .description("Associated project ID")
        .add_slot("impact_factor")
        .range("decimal")
        .minimum_value(0.0)
        .maximum_value(200.0)
        .description("Journal impact factor")
        .add_slot("citations")
        .range("integer")
        .minimum_value(0)
        .description("Number of citations");

    // Funding Source class
    builder
        .add_class("FundingSource")
        .description("Source of research funding")
        .add_slot("funding_id")
        .range("string")
        .required(true)
        .identifier(true)
        .add_slot("agency")
        .range("string")
        .required(true)
        .enum_values(vec!["NSF", "NIH", "DOE", "NASA", "Private", "Internal"])
        .add_slot("grant_number")
        .range("string")
        .required(true)
        .add_slot("amount")
        .range("decimal")
        .required(true)
        .minimum_value(0.0)
        .unit("USD")
        .add_slot("project_id")
        .range("string")
        .required(true);

    // Add example instances for visualization
    builder.add_example(
        "ResearchProject",
        serde_json::json!({
            "project_id": "PROJ-2024-001",
            "title": "Advanced Machine Learning for Climate Prediction",
            "status": "active",
            "start_date": "2024-01-15",
            "budget": 1500000.0,
            "completion_percentage": 35
        }),
    );

    builder.add_example(
        "TeamMember",
        serde_json::json!({
            "member_id": "TM-000123",
            "name": "Dr. Jane Smith",
            "email": "jane.smith@university.edu",
            "role": "PI",
            "hours_per_week": 20.0,
            "project_id": "PROJ-2024-001"
        }),
    );

    builder.add_example(
        "Publication",
        serde_json::json!({
            "pub_id": "PUB-2024-0042",
            "title": "Neural Networks for Weather Pattern Recognition",
            "authors": ["Smith, J.", "Johnson, R.", "Chen, L."],
            "publication_type": "journal",
            "year": 2024,
            "doi": "10.1234/example.2024.042",
            "project_id": "PROJ-2024-001",
            "impact_factor": 8.5,
            "citations": 12
        }),
    );

    builder.build()
}

fn generate_variants(schema: &Schema) -> Result<()> {
    println!("
📁 Generating Excel variants...");

    // 1. Simple single-sheet version
    let simple_options = ExcelOptions {
        separate_sheets: false,
        use_conditional_formatting: false,
        include_charts: false,
        ..Default::default()
    };

    let generator = ExcelGenerator::new();
    generator.generate_file(schema, "research_simple.xlsx", Some(simple_options))?;
    println!("  ✅ research_simple.xlsx - Basic single sheet");

    // 2. Data entry template
    let template_options = ExcelOptions {
        separate_sheets: true,
        sheet_per_class: true,
        add_dropdown_validation: true,
        add_range_validation: true,
        highlight_required_fields: true,
        include_examples: true,
        add_tooltips: true,
        freeze_headers: true,
        ..Default::default()
    };

    generator.generate_file(schema, "research_template.xlsx", Some(template_options))?;
    println!("  ✅ research_template.xlsx - Data entry template");

    // 3. Analysis workbook
    let analysis_options = ExcelOptions {
        separate_sheets: true,
        include_charts: true,
        add_pivot_tables: true,
        create_dashboard_sheet: true,
        use_table_styles: true,
        ..Default::default()
    };

    generator.generate_file(schema, "research_analysis.xlsx", Some(analysis_options))?;
    println!("  ✅ research_analysis.xlsx - Analysis workbook");

    // 4. Documentation workbook
    let docs_options = ExcelOptions {
        add_schema_documentation: true,
        include_examples: true,
        add_tooltips: true,
        color_code_by_type: true,
        ..Default::default()
    };

    generator.generate_file(schema, "research_docs.xlsx", Some(docs_options))?;
    println!("  ✅ research_docs.xlsx - Documentation focus");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_excel_generation() {
        let schema = create_research_schema()?;
        let generator = ExcelGenerator::new();

        // Test basic generation
        let result = generator.generate(&schema, None);
        assert!(result.is_ok());

        // Test with all options enabled
        let options = ExcelOptions {
            separate_sheets: true,
            use_conditional_formatting: true,
            include_charts: true,
            ..Default::default()
        };

        let result = generator.generate(&schema, Some(options));
        assert!(result.is_ok());
    }
}
