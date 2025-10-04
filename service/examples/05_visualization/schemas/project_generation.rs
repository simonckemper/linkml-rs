//! Example of generating complete projects from LinkML schemas

use linkml_service::generator::{
    LicenseType, ProjectGenerator, ProjectGeneratorConfig, ProjectTarget,
};
use linkml_service::parser::SchemaParser;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Example LinkML schema for a data model
    let schema_yaml = r#"
id: https://example.com/my-data-model
name: MyDataModel
description: A comprehensive data model for scientific research data

prefixes:
  linkml: https://w3id.org/linkml/
  schema: https://schema.org/
  ex: https://example.com/ns/

default_prefix: ex

imports:
  - linkml:types

classes:
  Study:
    description: A scientific research study
    attributes:
      id:
        description: Unique identifier for the study
        identifier: true
        range: string
      title:
        description: Title of the study
        required: true
        range: string
      description:
        description: Detailed description of the study
        range: string
      start_date:
        description: When the study began
        range: date
      end_date:
        description: When the study ended
        range: date
      principal_investigator:
        description: Lead researcher
        range: Person
        required: true
      participants:
        description: Study participants
        range: Person
        multivalued: true
      datasets:
        description: Datasets produced by the study
        range: Dataset
        multivalued: true

  Person:
    description: A person involved in research
    attributes:
      orcid:
        description: ORCID identifier
        identifier: true
        pattern: '^\d{4}-\d{4}-\d{4}-\d{3}[0-9X]$'
      name:
        description: Full name
        required: true
        range: string
      email:
        description: Email address
        range: string
        pattern: '^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$'
      affiliation:
        description: Institutional affiliation
        range: Organization

  Organization:
    description: An organization or institution
    attributes:
      ror_id:
        description: ROR (Research Organization Registry) ID
        identifier: true
        pattern: '^https://ror.org/0[a-z0-9]{8}$'
      name:
        description: Organization name
        required: true
        range: string
      type:
        description: Type of organization
        range: OrganizationType

  Dataset:
    description: A dataset produced by research
    attributes:
      doi:
        description: Digital Object Identifier
        identifier: true
        pattern: '^10\.\d{4,}/[-._;()/:\w]+$'
      title:
        description: Dataset title
        required: true
        range: string
      description:
        description: Dataset description
        range: string
      size_bytes:
        description: Size of dataset in bytes
        range: integer
        minimum_value: 0
      format:
        description: Data format
        range: string
      license:
        description: Data license
        range: LicenseType

enums:
  OrganizationType:
    description: Types of organizations
    permissible_values:
      university:
        description: Academic university
      research_institute:
        description: Research institute
      government:
        description: Government organization
      company:
        description: Commercial company
      nonprofit:
        description: Non-profit organization

  LicenseType:
    description: Data licensing options
    permissible_values:
      cc0:
        description: Creative Commons Zero
      cc_by:
        description: Creative Commons Attribution
      cc_by_sa:
        description: Creative Commons Attribution-ShareAlike
      cc_by_nc:
        description: Creative Commons Attribution-NonCommercial
      proprietary:
        description: Proprietary license
"#;

    // Parse the schema
    let mut parser = SchemaParser::new();
    let schema = parser.parse(schema_yaml)?;

    println!(
        "Generating projects for research data model...
"
    );

    // Generate Python project
    let python_config = ProjectGeneratorConfig {
        project_name: Some("research-data-model".to_string()),
        target: ProjectTarget::Python,
        include_docker: true,
        include_ci: true,
        include_tests: true,
        include_docs: true,
        include_examples: true,
        license: LicenseType::Mit,
        author: Some("Research Team".to_string()),
        author_email: Some("team@research.org".to_string()),
        version: "0.1.0".to_string(),
        ..Default::default()
    };

    let python_generator = ProjectGenerator::new(python_config);
    let python_manifest = python_generator.generate(&schema)?;

    println!("=== Python Project Structure ===");
    println!("{}", &python_manifest[..python_manifest.len().min(2000)]);
    println!(
        "... (truncated)
"
    );

    // Generate TypeScript project
    let typescript_config = ProjectGeneratorConfig {
        project_name: Some("research-data-model".to_string()),
        target: ProjectTarget::TypeScript,
        include_docker: true,
        include_ci: true,
        include_tests: true,
        license: LicenseType::Mit,
        author: Some("Research Team".to_string()),
        author_email: Some("team@research.org".to_string()),
        ..Default::default()
    };

    let typescript_generator = ProjectGenerator::new(typescript_config);
    let typescript_manifest = typescript_generator.generate(&schema)?;

    println!("=== TypeScript Project Structure ===");
    // Show just the file list
    if let Some(structure_start) = typescript_manifest.find("## Project Structure") {
        if let Some(structure_end) =
            typescript_manifest[structure_start..].find("## Generated Files")
        {
            println!(
                "{}",
                &typescript_manifest[structure_start..structure_start + structure_end]
            );
        }
    }

    // Generate Rust project
    let rust_config = ProjectGeneratorConfig {
        project_name: Some("research-data-model".to_string()),
        target: ProjectTarget::Rust,
        include_docker: false,
        include_ci: true,
        license: LicenseType::Apache2,
        author: Some("Research Team".to_string()),
        ..Default::default()
    };

    let rust_generator = ProjectGenerator::new(rust_config);
    let rust_manifest = rust_generator.generate(&schema)?;

    println!(
        "
=== Rust Project Structure ==="
    );
    if let Some(structure_start) = rust_manifest.find("## Project Structure") {
        if let Some(structure_end) = rust_manifest[structure_start..].find("## Generated Files") {
            println!(
                "{}",
                &rust_manifest[structure_start..structure_start + structure_end]
            );
        }
    }

    // Generate multi-language project
    let multi_config = ProjectGeneratorConfig {
        project_name: Some("research-data-model-multi".to_string()),
        target: ProjectTarget::MultiLanguage,
        include_docker: true,
        include_ci: true,
        license: LicenseType::Apache2,
        ..Default::default()
    };

    let multi_generator = ProjectGenerator::new(multi_config);
    let multi_manifest = multi_generator.generate(&schema)?;

    println!(
        "
=== Multi-Language Project Structure ==="
    );
    if let Some(structure_start) = multi_manifest.find("## Project Structure") {
        if let Some(structure_end) = multi_manifest[structure_start..].find("## Generated Files") {
            println!(
                "{}",
                &multi_manifest[structure_start..structure_start + structure_end]
            );
        }
    }

    // Save manifests to files
    std::fs::write("python_project_manifest.txt", &python_manifest)?;
    std::fs::write("typescript_project_manifest.txt", &typescript_manifest)?;
    std::fs::write("rust_project_manifest.txt", &rust_manifest)?;
    std::fs::write("multi_project_manifest.txt", &multi_manifest)?;

    println!(
        "
Project manifests saved to:"
    );
    println!("  - python_project_manifest.txt");
    println!("  - typescript_project_manifest.txt");
    println!("  - rust_project_manifest.txt");
    println!("  - multi_project_manifest.txt");
    println!(
        "
Note: The manifest shows what files would be generated. In a real scenario,"
    );
    println!("these files would be written to disk in the appropriate directory structure.");

    Ok(())
}
