//! Schema generation example for LinkML service
//!
//! This example demonstrates:
//! - Generating code from LinkML schemas
//! - TypeQL schema generation
//! - SQL DDL generation
//! - GraphQL schema generation
//! - Rust struct generation
//! - Documentation generation

use linkml_service::{create_linkml_service, LinkMLService};
use linkml_core::{prelude::*, error::Result};
use std::path::PathBuf;
use std::fs;
use std::sync::Arc;

// RootReal service dependencies
use logger_core::LoggerService;
use timestamp_core::TimestampService;
use task_management_service::StandardTaskManagementService;
use error_handling_service::StandardErrorHandlingService;
use configuration_service::StandardConfigurationService;
use cache_service::ValkeyCache;
use monitoring_service::StandardMonitoringService;

#[tokio::main]
async fn main() -> Result<()> {
    println!("LinkML Schema Generation Example");
    println!("===============================\n");

    // Initialize RootReal services
    let logger: Arc<dyn LoggerService<Error = logger_core::LoggerError>> =
        Arc::new(logger_service::StandardLoggerService::new()?);
    let timestamp: Arc<dyn TimestampService<Error = timestamp_core::TimestampError>> =
        Arc::new(timestamp_service::StandardTimestampService::new()?);

    // Non-dyn-compatible services use concrete types
    let task_manager = Arc::new(StandardTaskManagementService::new()?);
    let error_handler = Arc::new(StandardErrorHandlingService::new(
        logger.clone(),
        timestamp.clone(),
    )?);
    let config_service = Arc::new(StandardConfigurationService::new()?);

    // Dyn-compatible services
    let cache: Arc<dyn cache_core::CacheService<Error = cache_core::CacheError>> =
        Arc::new(ValkeyCache::new(
            cache_core::CacheConfig::default(),
            logger.clone(),
            Arc::new(container_management_service::StandardContainerManagementService::new()?),
            task_manager.clone(),
            Arc::new(memory_service::StandardMemoryService::new()?),
        ).await?);

    let monitor: Arc<dyn monitoring_core::MonitoringService<Error = monitoring_core::MonitoringError>> =
        Arc::new(StandardMonitoringService::new(
            logger.clone(),
            timestamp.clone(),
            task_manager.clone(),
        )?);

    let service = create_linkml_service(
        logger,
        timestamp,
        task_manager,
        error_handler,
        config_service,
        cache,
        monitor,
    ).await?;

    // Define a comprehensive schema
    let schema_yaml = r#"
id: https://example.org/library-schema
name: LibrarySchema
description: Schema for a library management system
version: "2.0.0"
license: MIT

prefixes:
  library: https://example.org/library/
  schema: http://schema.org/

default_prefix: library

classes:
  LibraryItem:
    abstract: true
    description: Base class for all library items
    slots:
      - id
      - title
      - publication_date
      - isbn
      - status

  Book:
    is_a: LibraryItem
    description: A physical or digital book
    class_uri: schema:Book
    slots:
      - authors
      - publisher
      - edition
      - pages
      - genres

  Journal:
    is_a: LibraryItem
    description: Academic or professional journal
    slots:
      - issn
      - volume
      - issue
      - publisher
      - impact_factor

  Member:
    description: Library member who can borrow items
    tree_root: true
    slots:
      - member_id
      - name
      - email
      - phone
      - address
      - membership_type
      - join_date
      - borrowed_items

  Loan:
    description: A loan transaction
    slots:
      - loan_id
      - member
      - item
      - loan_date
      - due_date
      - return_date
      - fine_amount

  Author:
    description: Author of books or articles
    slots:
      - author_id
      - full_name
      - biography
      - birth_date
      - nationality

  Publisher:
    description: Publishing company
    slots:
      - publisher_id
      - name
      - founded_year
      - headquarters
      - website

slots:
  id:
    identifier: true
    range: string
    required: true

  title:
    slot_uri: schema:name
    range: string
    required: true

  publication_date:
    range: date

  isbn:
    description: International Standard Book Number
    range: string
    pattern: "^(97[89])?\\d{10}$"

  status:
    range: ItemStatus
    required: true

  authors:
    range: Author
    multivalued: true
    minimum_cardinality: 1

  publisher:
    range: Publisher

  edition:
    range: integer
    minimum_value: 1

  pages:
    range: integer
    minimum_value: 1

  genres:
    range: Genre
    multivalued: true

  issn:
    description: International Standard Serial Number
    range: string
    pattern: "^\\d{4}-\\d{4}$"

  volume:
    range: integer
    minimum_value: 1

  issue:
    range: integer
    minimum_value: 1

  impact_factor:
    range: float
    minimum_value: 0

  member_id:
    identifier: true
    range: string
    pattern: "^MEM\\d{6}$"
    required: true

  name:
    range: string
    required: true

  email:
    slot_uri: schema:email
    range: string
    pattern: "^[\\w._%+-]+@[\\w.-]+\\.[A-Z|a-z]{2,}$"
    required: true

  phone:
    range: string
    pattern: "^\\+?[\\d\\s()-]+$"

  address:
    range: string

  membership_type:
    range: MembershipType
    required: true

  join_date:
    range: date
    required: true

  borrowed_items:
    range: Loan
    multivalued: true
    inlined: false

  loan_id:
    identifier: true
    range: string
    required: true

  member:
    range: Member
    required: true

  item:
    range: LibraryItem
    required: true

  loan_date:
    range: datetime
    required: true

  due_date:
    range: datetime
    required: true

  return_date:
    range: datetime

  fine_amount:
    range: decimal
    minimum_value: 0
    unit:
      symbol: USD

  author_id:
    identifier: true
    range: string
    required: true

  full_name:
    range: string
    required: true

  biography:
    range: string

  birth_date:
    range: date

  nationality:
    range: string

  publisher_id:
    identifier: true
    range: string
    required: true

  founded_year:
    range: integer
    minimum_value: 1000
    maximum_value: 2100

  headquarters:
    range: string

  website:
    range: uri

enums:
  ItemStatus:
    permissible_values:
      available:
        description: Item is available for loan
      borrowed:
        description: Item is currently on loan
      reserved:
        description: Item is reserved
      maintenance:
        description: Item is under maintenance
      lost:
        description: Item is lost

  MembershipType:
    permissible_values:
      student:
        description: Student membership
      faculty:
        description: Faculty membership
      public:
        description: Public membership
      premium:
        description: Premium membership with extra benefits

  Genre:
    permissible_values:
      fiction:
        description: Fiction books
      non_fiction:
        description: Non-fiction books
      science:
        description: Science and technology
      history:
        description: Historical works
      biography:
        description: Biographical works
      reference:
        description: Reference materials
"#;

    let schema = service.load_schema_str(schema_yaml, SchemaFormat::Yaml).await?;
    println!("✓ Schema loaded: {}\n", schema.name);

    // Create output directory
    let output_dir = PathBuf::from("generated");
    fs::create_dir_all(&output_dir)?;

    // Example 1: Generate TypeQL schema
    println!("1. Generating TypeQL schema:");
    use linkml_service::generator::{TypeQLGenerator, Generator, GeneratorOptions};
    let typeql_gen = TypeQLGenerator::new();
    let typeql = typeql_gen.generate(&schema, &GeneratorOptions::default()).await?;
    let typeql_path = output_dir.join("library_schema.tql");
    fs::write(&typeql_path, &typeql)?;
    println!("   ✓ Generated: {}", typeql_path.display());
    println!("   Preview:");
    println!("{}", &typeql[..500.min(typeql.len())]);
    println!("   ...\n");

    // Example 2: Generate SQL DDL
    println!("2. Generating SQL DDL:");
    use linkml_service::generator::{SQLGenerator, sql::SqlDialect};
    let sql_gen = SQLGenerator::new();
    let sql = sql_gen.generate_with_dialect(&schema, SqlDialect::PostgreSQL).await?;
    let sql_path = output_dir.join("library_schema.sql");
    fs::write(&sql_path, &sql)?;
    println!("   ✓ Generated: {}", sql_path.display());
    println!("   Preview:");
    println!("{}", &sql[..500.min(sql.len())]);
    println!("   ...\n");

    // Example 3: Generate GraphQL schema
    println!("3. Generating GraphQL schema:");
    use linkml_service::generator::GraphQLGenerator;
    let graphql_gen = GraphQLGenerator::new();
    let graphql = graphql_gen.generate(&schema, &GeneratorOptions::default()).await?;
    let graphql_path = output_dir.join("library_schema.graphql");
    fs::write(&graphql_path, &graphql)?;
    println!("   ✓ Generated: {}", graphql_path.display());
    println!("   Preview:");
    println!("{}", &graphql[..500.min(graphql.len())]);
    println!("   ...\n");

    // Example 4: Generate Rust structs
    println!("4. Generating Rust code:");
    use linkml_service::generator::RustGenerator;
    let rust_gen = RustGenerator::new();
    let mut rust_options = GeneratorOptions::default();
    // Configure Rust-specific options if needed
    let rust_code = rust_gen.generate(&schema, &rust_options).await?;
    let rust_path = output_dir.join("library_schema.rs");
    fs::write(&rust_path, &rust_code)?;
    println!("   ✓ Generated: {}", rust_path.display());
    println!("   Preview:");
    println!("{}", &rust_code[..500.min(rust_code.len())]);
    println!("   ...\n");

    // Example 5: Generate Python dataclasses
    println!("5. Generating Python dataclasses:");
    // Note: Python generation would require a Python generator implementation
    println!("   (Python generation not implemented in this example)");
    /*
    let python_code = service.generate_python(&schema, PythonGenerationOptions {
        use_dataclasses: true,
        use_pydantic: false,
        generate_validators: true,
        ..Default::default()
    }).await?;
    let python_path = output_dir.join("library_schema.py");
    fs::write(&python_path, &python_code)?;
    println!("   ✓ Generated: {}", python_path.display());
    */

    // Example 6: Generate documentation
    println!("\n6. Generating documentation:");

    // Markdown documentation
    use linkml_service::generator::doc::DocGenerator;
    use linkml_core::traits::DocFormat;
    let doc_gen = DocGenerator::new();
    let markdown = doc_gen.generate_format(&schema, DocFormat::Markdown).await?;
    let md_path = output_dir.join("library_schema.md");
    fs::write(&md_path, &markdown)?;
    println!("   ✓ Generated Markdown: {}", md_path.display());

    // HTML documentation
    use linkml_service::generator::HtmlGenerator;
    let html_gen = HtmlGenerator::new();
    let html = html_gen.generate(&schema, &GeneratorOptions::default()).await?;
    let html_path = output_dir.join("library_schema.html");
    fs::write(&html_path, &html)?;
    println!("   ✓ Generated HTML: {}", html_path.display());

    // JSON-LD context
    // Note: JSON-LD generation would require a specific generator implementation
    println!("\n   (JSON-LD generation not implemented in this example)");
    /*
    let jsonld = service.generate_jsonld_context(&schema).await?;
    let jsonld_path = output_dir.join("library_context.jsonld");
    fs::write(&jsonld_path, &jsonld)?;
    println!("   ✓ Generated JSON-LD: {}", jsonld_path.display());
    */

    // Example 7: Generate validation functions
    println!("\n7. Generating validation functions:");
    // Note: Validator generation would require specific language generators
    println!("   (Validator generation not implemented in this example)");
    /*
    let validators = service.generate_validators(&schema, ValidatorLanguage::TypeScript).await?;
    let validators_path = output_dir.join("library_validators.ts");
    fs::write(&validators_path, &validators)?;
    println!("   ✓ Generated TypeScript validators: {}", validators_path.display());
    */

    // Example 8: Generate OpenAPI specification
    println!("\n8. Generating OpenAPI specification:");
    use linkml_service::generator::OpenApiGenerator;
    let openapi_gen = OpenApiGenerator::new();
    let mut openapi_options = GeneratorOptions::default();
    // Configure OpenAPI-specific options if needed
    let openapi = openapi_gen.generate(&schema, &openapi_options).await?;
    /*
    let openapi = service.generate_openapi(&schema, OpenApiOptions {
        title: "Library Management API".to_string(),
        version: "2.0.0".to_string(),
        base_path: "/api/v2".to_string(),
        include_crud_operations: true,
        ..Default::default()
    }).await?;
    */
    let openapi_path = output_dir.join("library_openapi.yaml");
    fs::write(&openapi_path, &openapi)?;
    println!("   ✓ Generated OpenAPI spec: {}", openapi_path.display());

    // Example 9: Generate example data
    println!("\n9. Generating example data:");
    // Note: Example generation would require specific implementation
    println!("   (Example generation not implemented in this example)");
    /*
    let examples = service.generate_examples(&schema, ExampleGenerationOptions {
        count_per_class: 3,
        include_edge_cases: true,
        seed: Some(42),
        ..Default::default()
    }).await?;
    let examples_path = output_dir.join("library_examples.json");
    fs::write(&examples_path, serde_json::to_string_pretty(&examples)?)?;
    println!("   ✓ Generated examples: {}", examples_path.display());
    */

    // Example 10: Generate migration scripts
    println!("\n10. Generating migration scripts:");
    // Note: Migration generation would require specific implementation
    println!("   (Migration generation not implemented in this example)");

    /*
    // Suppose we have a previous version
    let old_schema_yaml = r#"
id: https://example.org/library-schema
name: LibrarySchema
version: "1.0.0"

classes:
  Book:
    slots:
      - id
      - title
      - isbn

  Member:
    slots:
      - member_id
      - name
      - email

slots:
  id:
    identifier: true
    range: string

  title:
    range: string
    required: true

  isbn:
    range: string

  member_id:
    identifier: true
    range: string

  name:
    range: string

  email:
    range: string
"#;

    let old_schema = service.load_schema_str(old_schema_yaml, SchemaFormat::Yaml).await?;
    let migration = service.generate_migration(&old_schema, &schema, MigrationOptions {
        target_language: MigrationLanguage::SQL,
        include_data_migration: true,
        safe_mode: true,
        ..Default::default()
    }).await?;

    let migration_path = output_dir.join("migrate_v1_to_v2.sql");
    fs::write(&migration_path, &migration)?;
    println!("   ✓ Generated migration script: {}", migration_path.display());
    */

    println!("\n✓ All files generated successfully in '{}'", output_dir.display());
    println!("\nGenerated files summary:");
    println!("  - TypeQL schema: library_schema.tql");
    println!("  - SQL DDL: library_schema.sql");
    println!("  - GraphQL schema: library_schema.graphql");
    println!("  - Rust code: library_schema.rs");
    println!("  - Markdown docs: library_schema.md");
    println!("  - HTML docs: library_schema.html");
    println!("  - OpenAPI spec: library_openapi.yaml");

    Ok(())
}
