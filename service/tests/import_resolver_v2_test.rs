//! Comprehensive tests for enhanced import resolution

use linkml_core::settings::{ImportResolutionStrategy, ImportSettings};
use linkml_service::parser::{ImportResolverV2, SchemaLoader, SchemaParser, YamlParser};
use tempfile::TempDir;
use tokio::fs;

#[tokio::test]
async fn test_basic_file_import() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path();

    // Create base schema
    let base_schema = r#"
id: https://example.org/base
name: base
classes:
  BaseClass:
    name: BaseClass
    description: A base class
    slots:
      - name
slots:
  name:
    name: name
    range: string
    required: true
"#;

    fs::write(base_path.join("base.yaml"), base_schema)
        .await
        ?;

    // Create main schema that imports base
    let main_schema = r#"
id: https://example.org/main
name: main
imports:
  - base
classes:
  Person:
    name: Person
    is_a: BaseClass
    description: A person class
    slots:
      - age
slots:
  age:
    name: age
    range: integer
"#;

    fs::write(base_path.join("main.yaml"), main_schema)
        .await
        ?;

    // Load and resolve
    let loader = SchemaLoader::new();
    let schema = loader
        .load(base_path.join("main.yaml"))
        .await
        ?;

    // Verify imports were resolved
    assert!(schema.classes.contains_key("BaseClass"));
    assert!(schema.classes.contains_key("Person"));
    assert!(schema.slots.contains_key("name"));
    assert!(schema.slots.contains_key("age"));

    // Verify inheritance
    let person = &schema.classes["Person"];
    assert_eq!(person.is_a, Some("BaseClass".to_string()));
    Ok(())
}

#[tokio::test]
async fn test_import_aliases() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path();

    // Create schemas in subdirectory
    let schemas_dir = base_path.join("schemas");
    fs::create_dir_all(&schemas_dir)
        .await
        ?;

    let common_schema = r#"
id: https://example.org/common
name: common
types:
  Identifier:
    name: Identifier
    typeof: string
    pattern: "^[A-Z][0-9]+$"
"#;

    fs::write(schemas_dir.join("common.yaml"), common_schema)
        .await
        ?;

    // Main schema with import alias
    let main_schema = r#"
id: https://example.org/main
name: main
settings:
  imports:
    aliases:
      common: schemas/common
imports:
  - common
classes:
  Entity:
    name: Entity
    slots:
      - id
slots:
  id:
    name: id
    range: Identifier
"#;

    let parser = YamlParser::new();
    let schema = parser
        .parse(main_schema)
        ?;

    let mut settings = ImportSettings::default();
    settings.search_paths.push(
        base_path
            .to_str()
            ?
            .to_string(),
    );
    // No need to add alias since it's in the schema settings

    let resolver = ImportResolverV2::with_settings(settings);
    let resolved = resolver
        .resolve_imports(&schema)
        .await
        ?;

    // Verify alias resolution
    assert!(resolved.types.contains_key("Identifier"));
    assert_eq!(
        resolved.types["Identifier"].pattern,
        Some("^[A-Z][0-9]+$".to_string())
    );
    Ok(())
}

#[tokio::test]
async fn test_selective_imports() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path();

    // Create a large schema with many elements
    let library_schema = r#"
id: https://example.org/library
name: library
classes:
  Book:
    name: Book
    description: A book
  Author:
    name: Author
    description: An author
  Publisher:
    name: Publisher
    description: A publisher
  Library:
    name: Library
    description: A library
types:
  ISBN:
    name: ISBN
    typeof: string
  PublicationYear:
    name: PublicationYear
    typeof: integer
enums:
  BookGenre:
    name: BookGenre
    permissible_values:
      - text: fiction
      - text: nonfiction
  BookStatus:
    name: BookStatus
    permissible_values:
      - text: available
      - text: checked_out
"#;

    fs::write(base_path.join("library.yaml"), library_schema)
        .await
        ?;

    // For now, we'll test basic import functionality
    // TODO: Implement selective import syntax parsing (e.g., "library[Book,Author]")
    let main_schema = r#"
id: https://example.org/catalog
name: catalog
imports:
  - library
classes:
  BookCatalog:
    name: BookCatalog
    slots:
      - books
slots:
  books:
    name: books
    range: Book
    multivalued: true
"#;

    let parser = YamlParser::new();
    let schema = parser
        .parse(main_schema)
        ?;

    let mut settings = ImportSettings::default();
    settings.search_paths.push(
        base_path
            .to_str()
            ?
            .to_string(),
    );

    let resolver = ImportResolverV2::with_settings(settings);
    let resolved = resolver
        .resolve_imports(&schema)
        .await
        ?;

    // All elements should be imported for now
    assert!(resolved.classes.contains_key("Book"));
    assert!(resolved.classes.contains_key("Author"));
    assert!(resolved.types.contains_key("ISBN"));
    assert!(resolved.enums.contains_key("BookGenre"));
    Ok(())
}

#[tokio::test]
async fn test_conflict_resolution() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path();

    // Schema A with Status class
    let schema_a = r#"
id: https://example.org/a
name: a
classes:
  Status:
    name: Status
    description: Status from schema A
    slots:
      - code
      - message
slots:
  code:
    name: code
    range: integer
  message:
    name: message
    range: string
"#;

    fs::write(base_path.join("a.yaml"), schema_a)
        .await
        ?;

    // Schema B with different Status class
    let schema_b = r#"
id: https://example.org/b
name: b
classes:
  Status:
    name: Status
    description: Status from schema B
    slots:
      - state
      - timestamp
slots:
  state:
    name: state
    range: string
  timestamp:
    name: timestamp
    range: datetime
"#;

    fs::write(base_path.join("b.yaml"), schema_b)
        .await
        ?;

    // Main schema importing both
    let main_schema = r#"
id: https://example.org/main
name: main
imports:
  - a
  - b
classes:
  System:
    name: System
    description: Uses both status types
"#;

    let parser = YamlParser::new();
    let schema = parser
        .parse(main_schema)
        ?;

    let mut settings = ImportSettings::default();
    settings.search_paths.push(
        base_path
            .to_str()
            ?
            .to_string(),
    );

    let resolver = ImportResolverV2::with_settings(settings);
    let resolved = resolver
        .resolve_imports(&schema)
        .await
        ?;

    // Should have Status from first import and qualified name for second
    assert!(resolved.classes.contains_key("Status"));
    assert!(resolved.classes.contains_key("b_Status"));

    // Check that both versions have their correct slots
    assert!(resolved.slots.contains_key("code"));
    assert!(resolved.slots.contains_key("message"));
    assert!(resolved.slots.contains_key("state"));
    assert!(resolved.slots.contains_key("timestamp"));
    Ok(())
}

#[tokio::test]
async fn test_import_with_prefix() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path();

    // Create a schema to be imported with prefix
    let geo_schema = r#"
id: https://example.org/geo
name: geo
classes:
  Location:
    name: Location
    slots:
      - latitude
      - longitude
  Address:
    name: Address
    slots:
      - street
      - city
slots:
  latitude:
    name: latitude
    range: float
  longitude:
    name: longitude
    range: float
  street:
    name: street
    range: string
  city:
    name: city
    range: string
"#;

    fs::write(base_path.join("geo.yaml"), geo_schema)
        .await
        ?;

    // For now, test basic import without prefix
    // TODO: Implement prefix syntax parsing (e.g., "geo as geo_")
    let main_schema = r#"
id: https://example.org/app
name: app
imports:
  - geo
classes:
  Store:
    name: Store
    slots:
      - location
slots:
  location:
    name: location
    range: Location
"#;

    let parser = YamlParser::new();
    let schema = parser
        .parse(main_schema)
        ?;

    let mut settings = ImportSettings::default();
    settings.search_paths.push(
        base_path
            .to_str()
            ?
            .to_string(),
    );

    let resolver = ImportResolverV2::with_settings(settings);
    let resolved = resolver
        .resolve_imports(&schema)
        .await
        ?;

    // Verify import worked
    assert!(resolved.classes.contains_key("Location"));
    assert!(resolved.classes.contains_key("Address"));
    Ok(())
}

#[tokio::test]
async fn test_import_resolution_strategies() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path();

    // Create directory structure
    let lib_dir = base_path.join("lib");
    let local_dir = base_path.join("local");
    fs::create_dir_all(&lib_dir)
        .await
        ?;
    fs::create_dir_all(&local_dir)
        .await
        ?;

    // Same filename in different locations
    let lib_common = r#"
id: https://example.org/lib/common
name: common
description: Library version
types:
  Version:
    name: Version
    typeof: string
    pattern: "lib-v[0-9]+"
"#;

    fs::write(lib_dir.join("common.yaml"), lib_common)
        .await
        ?;

    let local_common = r#"
id: https://example.org/local/common
name: common
description: Local version
types:
  Version:
    name: Version
    typeof: string
    pattern: "local-v[0-9]+"
"#;

    fs::write(local_dir.join("common.yaml"), local_common)
        .await
        ?;

    // Test relative strategy (should find local first)
    let main_schema = r#"
id: https://example.org/main
name: main
imports:
  - common
"#;

    fs::write(local_dir.join("main.yaml"), main_schema)
        .await
        ?;

    let parser = YamlParser::new();
    let schema = parser
        .parse(main_schema)
        ?;

    // Test relative resolution
    let mut settings = ImportSettings::default();
    settings.search_paths = vec![
        local_dir
            .to_str()
            ?
            .to_string(),
        lib_dir.to_str()?.to_string(),
    ];
    settings.resolution_strategy = Some(ImportResolutionStrategy::Relative);

    let resolver = ImportResolverV2::with_settings(settings.clone());
    let resolved = resolver
        .resolve_imports(&schema)
        .await
        ?;

    // Should get local version with relative strategy
    assert!(resolved.types.contains_key("Version"));
    assert_eq!(
        resolved.types["Version"].pattern,
        Some("local-v[0-9]+".to_string())
    );

    // Test absolute resolution (search paths only)
    settings.resolution_strategy = Some(ImportResolutionStrategy::Absolute);
    settings.search_paths = vec![lib_dir.to_str()?.to_string()];

    let resolver = ImportResolverV2::with_settings(settings);
    let resolved = resolver
        .resolve_imports(&schema)
        .await
        ?;

    // Should get lib version with absolute strategy
    assert!(resolved.types.contains_key("Version"));
    assert_eq!(
        resolved.types["Version"].pattern,
        Some("lib-v[0-9]+".to_string())
    );
    Ok(())
}

#[tokio::test]
async fn test_deep_import_chain() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path();

    // Create a chain of imports: main -> middle -> base
    let base_schema = r#"
id: https://example.org/base
name: base
types:
  BaseType:
    name: BaseType
    typeof: string
"#;

    fs::write(base_path.join("base.yaml"), base_schema)
        .await
        ?;

    let middle_schema = r#"
id: https://example.org/middle
name: middle
imports:
  - base
classes:
  MiddleClass:
    name: MiddleClass
    slots:
      - base_field
slots:
  base_field:
    name: base_field
    range: BaseType
"#;

    fs::write(base_path.join("middle.yaml"), middle_schema)
        .await
        ?;

    let main_schema = r#"
id: https://example.org/main
name: main
imports:
  - middle
classes:
  MainClass:
    name: MainClass
    is_a: MiddleClass
"#;

    fs::write(base_path.join("main.yaml"), main_schema)
        .await
        ?;

    let loader = SchemaLoader::new();
    let schema = loader
        .load(base_path.join("main.yaml"))
        .await
        ?;

    // Verify entire chain was resolved
    assert!(schema.types.contains_key("BaseType"));
    assert!(schema.classes.contains_key("MiddleClass"));
    assert!(schema.classes.contains_key("MainClass"));
    assert!(schema.slots.contains_key("base_field"));
    Ok(())
}

#[tokio::test]
async fn test_import_depth_limit() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path();

    // Create a deep chain that exceeds max depth
    for i in 0..15 {
        let schema = if i == 0 {
            format!(
                r#"
id: https://example.org/schema{}
name: schema{}
classes:
  Class{}:
    name: Class{}
"#,
                i, i, i, i
            )
        } else {
            format!(
                r#"
id: https://example.org/schema{}
name: schema{}
imports:
  - schema{}
classes:
  Class{}:
    name: Class{}
"#,
                i,
                i,
                i - 1,
                i,
                i
            )
        };

        fs::write(base_path.join(format!("schema{}.yaml", i)), schema)
            .await
            ?;
    }

    let parser = YamlParser::new();
    let schema = parser
        .parse(&format!(
            r#"
id: https://example.org/schema14
name: schema14
imports:
  - schema13
classes:
  Class14:
    name: Class14
"#
        ))
        ?;

    let mut settings = ImportSettings::default();
    settings.search_paths.push(
        base_path
            .to_str()
            ?
            .to_string(),
    );
    settings.max_import_depth = Some(5); // Set low limit

    let resolver = ImportResolverV2::with_settings(settings);
    let result = resolver.resolve_imports(&schema).await;

    // Should fail due to depth limit
    match result {
        Ok(_) => panic!("Expected error due to depth limit, but got success"),
        Err(err) => {
            assert!(
                err.to_string().contains("Maximum import depth")
                    || err.to_string().contains("exceeded"),
                "Expected depth limit error, got: {}",
                err
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn test_cache_behavior() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path();

    // Create a schema that's imported multiple times
    let shared_schema = r#"
id: https://example.org/shared
name: shared
types:
  SharedType:
    name: SharedType
    typeof: string
"#;

    fs::write(base_path.join("shared.yaml"), shared_schema)
        .await
        ?;

    // Two schemas that both import shared
    let schema_a = r#"
id: https://example.org/a
name: a
imports:
  - shared
classes:
  ClassA:
    name: ClassA
"#;

    fs::write(base_path.join("a.yaml"), schema_a)
        .await
        ?;

    let schema_b = r#"
id: https://example.org/b
name: b
imports:
  - shared
classes:
  ClassB:
    name: ClassB
"#;

    fs::write(base_path.join("b.yaml"), schema_b)
        .await
        ?;

    // Main imports both a and b (which both import shared)
    let main_schema = r#"
id: https://example.org/main
name: main
imports:
  - a
  - b
"#;

    let parser = YamlParser::new();
    let schema = parser
        .parse(main_schema)
        ?;

    let mut settings = ImportSettings::default();
    settings.search_paths.push(
        base_path
            .to_str()
            ?
            .to_string(),
    );
    settings.cache_imports = Some(true);

    let resolver = ImportResolverV2::with_settings(settings);
    let resolved = resolver
        .resolve_imports(&schema)
        .await
        ?;

    // Should have all elements without duplication
    assert!(resolved.types.contains_key("SharedType"));
    assert!(resolved.classes.contains_key("ClassA"));
    assert!(resolved.classes.contains_key("ClassB"));

    // Clear cache and resolve again
    resolver.clear_cache();
    let resolved2 = resolver
        .resolve_imports(&schema)
        .await
        ?;

    // Should get same result
    assert_eq!(resolved.types.len(), resolved2.types.len());
    assert_eq!(resolved.classes.len(), resolved2.classes.len());
    Ok(())
}

#[tokio::test]
async fn test_schema_settings_override() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path();

    // Create lib directory
    let lib_dir = base_path.join("lib");
    fs::create_dir_all(&lib_dir)
        .await
        ?;

    let lib_schema = r#"
id: https://example.org/lib
name: lib
classes:
  LibClass:
    name: LibClass
"#;

    fs::write(lib_dir.join("lib.yaml"), lib_schema)
        .await
        ?;

    // Schema with its own import settings
    let main_schema = r#"
id: https://example.org/main
name: main
settings:
  imports:
    search_paths:
      - lib
    follow_imports: true
    cache_imports: true
    resolution_strategy: absolute
imports:
  - lib
"#;

    fs::write(base_path.join("main.yaml"), main_schema)
        .await
        ?;

    let loader = SchemaLoader::new();
    let schema = loader
        .load(base_path.join("main.yaml"))
        .await
        ?;

    // Verify settings were used
    assert!(schema.classes.contains_key("LibClass"));
    Ok(())
}
