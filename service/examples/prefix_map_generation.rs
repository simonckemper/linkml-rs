//! Example of generating prefix maps from LinkML schemas

use linkml_service::generator::{
    Generator, PrefixMapFormat, PrefixMapGenerator, PrefixMapGeneratorConfig,
};
use linkml_service::parser::YamlParser;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Example LinkML schema with multiple namespace prefixes
    let schema_yaml = r#"
id: https://w3id.org/example/multi-namespace-schema
name: MultiNamespaceSchema
description: Schema demonstrating prefix management across multiple namespaces

prefixes:
  linkml: https://w3id.org/linkml/
  schema: https://schema.org/
  dcterms: http://purl.org/dc/terms/
  foaf: http://xmlns.com/foaf/0.1/
  ex: https://example.com/ns/
  bio: https://bioschemas.org/
  dcat: http://www.w3.org/ns/dcat#
  prov: http://www.w3.org/ns/prov#
  qudt: http://qudt.org/schema/qudt/

default_prefix: ex

imports:
  - linkml:types

classes:
  Dataset:
    description: A scientific dataset
    id_prefixes:
      - dcat
    attributes:
      title:
        description: Dataset title
        range: string
        slot_uri: dcterms:title
      creator:
        description: Dataset creator
        range: Person
        slot_uri: dcterms:creator
      issued:
        description: Date of formal issuance
        range: date
        slot_uri: dcterms:issued
      distribution:
        description: Available distributions
        range: Distribution
        multivalued: true

  Person:
    description: A person or agent
    id_prefixes:
      - foaf
    attributes:
      name:
        description: Full name
        range: string
        slot_uri: foaf:name
      email:
        description: Email address
        range: string
        slot_uri: foaf:mbox
      orcid:
        description: ORCID identifier
        range: uriorcurie
        pattern: '^https://orcid.org/\d{4}-\d{4}-\d{4}-\d{3}[0-9X]$'

  Distribution:
    description: A specific representation of a dataset
    id_prefixes:
      - dcat
    attributes:
      format:
        description: File format
        range: string
        slot_uri: dcterms:format
      access_url:
        description: URL to access the distribution
        range: uri
        slot_uri: dcat:accessURL
      byte_size:
        description: Size in bytes
        range: integer
        slot_uri: dcat:byteSize

  Measurement:
    description: A scientific measurement
    id_prefixes:
      - qudt
    attributes:
      value:
        description: Numeric value
        range: float
        required: true
      unit:
        description: Unit of measurement
        range: Unit
        required: true
      uncertainty:
        description: Measurement uncertainty
        range: float

  Unit:
    description: A unit of measurement
    id_prefixes:
      - qudt
    attributes:
      symbol:
        description: Unit symbol
        range: string
      dimension:
        description: Physical dimension
        range: string
"#;

    // Parse the schema
    let parser = YamlParser::new();
    let schema = parser.parse_str(schema_yaml)?;

    println!("Generating prefix maps in various formats...
");

    // Generate simple JSON format
    let config_simple = PrefixMapGeneratorConfig::default();
    let generator_simple = PrefixMapGenerator::new(config_simple);
    let output_simple = generator_simple.generate(&schema)?;

    println!("=== Simple JSON Format ===");
    println!("{}
", output_simple);

    // Generate extended JSON format with metadata
    let config_extended = PrefixMapGeneratorConfig {
        format: PrefixMapFormat::Extended,
        include_metadata: true,
        validate_prefixes: true,
        ..Default::default()
    };
    let generator_extended = PrefixMapGenerator::new(config_extended);
    let output_extended = generator_extended.generate(&schema)?;

    println!("=== Extended JSON Format with Metadata ===");
    println!("{}
", output_extended);

    // Generate Turtle format for SPARQL
    let config_turtle = PrefixMapGeneratorConfig {
        format: PrefixMapFormat::Turtle,
        ..Default::default()
    };
    let generator_turtle = PrefixMapGenerator::new(config_turtle);
    let output_turtle = generator_turtle.generate(&schema)?;

    println!("=== Turtle/SPARQL Format ===");
    println!("{}
", output_turtle);

    // Generate YAML format
    let config_yaml = PrefixMapGeneratorConfig {
        format: PrefixMapFormat::Yaml,
        include_metadata: true,
        ..Default::default()
    };
    let generator_yaml = PrefixMapGenerator::new(config_yaml);
    let output_yaml = generator_yaml.generate(&schema)?;

    println!("=== YAML Format ===");
    println!("{}
", output_yaml);

    // Generate CSV format
    let config_csv = PrefixMapGeneratorConfig {
        format: PrefixMapFormat::Csv,
        ..Default::default()
    };
    let generator_csv = PrefixMapGenerator::new(config_csv);
    let output_csv = generator_csv.generate(&schema)?;

    println!("=== CSV Format ===");
    println!("{}
", output_csv);

    // Generate with additional custom prefixes
    let mut additional_prefixes = std::collections::HashMap::new();
    additional_prefixes.insert(
        "custom".to_string(),
        "https://custom.example.com/".to_string(),
    );
    additional_prefixes.insert(
        "local".to_string(),
        "http://localhost:8080/vocab#".to_string(),
    );

    let config_custom = PrefixMapGeneratorConfig {
        format: PrefixMapFormat::Extended,
        include_metadata: true,
        additional_prefixes,
        ..Default::default()
    };
    let generator_custom = PrefixMapGenerator::new(config_custom);
    let output_custom = generator_custom.generate(&schema)?;

    println!("=== Extended Format with Custom Prefixes ===");
    println!("{}
", output_custom);

    // Save different formats to files
    std::fs::write("prefix_map.json", &output_simple)?;
    std::fs::write("prefix_map.ttl", &output_turtle)?;
    std::fs::write("prefix_map.yaml", &output_yaml)?;
    std::fs::write("prefix_map.csv", &output_csv)?;

    println!("Prefix maps saved to:");
    println!("  - prefix_map.json");
    println!("  - prefix_map.ttl");
    println!("  - prefix_map.yaml");
    println!("  - prefix_map.csv");

    Ok(())
}
