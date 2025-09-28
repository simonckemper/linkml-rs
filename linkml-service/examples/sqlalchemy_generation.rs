//! Example of generating SQLAlchemy ORM models from LinkML schemas

use linkml_service::generator::{Generator, SQLAlchemyGenerator, SQLAlchemyGeneratorConfig};
use linkml_service::parser::SchemaParser;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Example LinkML schema for a blog application
    let schema_yaml = r#"
id: https://example.com/blog-schema
name: BlogSchema
description: Schema for a simple blog application

prefixes:
  blog: https://example.com/blog/
  linkml: https://w3id.org/linkml/

default_prefix: blog

imports:
  - linkml:types

classes:
  User:
    description: A blog user
    attributes:
      username:
        description: Unique username
        range: string
        required: true
        identifier: true
      email:
        description: User's email address
        range: string
        required: true
        pattern: '^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$'
      full_name:
        description: User's full name
        range: string
      created_at:
        description: When the user account was created
        range: datetime
        required: true
      is_active:
        description: Whether the user account is active
        range: boolean
        default: true
    slots:
      - posts
      - comments

  Post:
    description: A blog post
    attributes:
      title:
        description: Post title
        range: string
        required: true
      slug:
        description: URL-friendly version of the title
        range: string
        required: true
        identifier: true
      content:
        description: Post content in markdown
        range: string
        required: true
      published_at:
        description: When the post was published
        range: datetime
      updated_at:
        description: When the post was last updated
        range: datetime
      status:
        description: Publication status
        range: PostStatus
        required: true
    slots:
      - author
      - tags
      - comments

  Comment:
    description: A comment on a blog post
    attributes:
      content:
        description: Comment text
        range: string
        required: true
      created_at:
        description: When the comment was created
        range: datetime
        required: true
      is_approved:
        description: Whether the comment has been approved
        range: boolean
        default: false
    slots:
      - author
      - post

  Tag:
    description: A tag for categorizing posts
    attributes:
      name:
        description: Tag name
        range: string
        required: true
        identifier: true
      description:
        description: Tag description
        range: string

slots:
  author:
    description: The author of a post or comment
    range: User
    required: true

  posts:
    description: Posts written by a user
    range: Post
    multivalued: true
    inverse: author

  comments:
    description: Comments made by a user
    range: Comment
    multivalued: true
    inverse: author

  post:
    description: The post a comment belongs to
    range: Post
    required: true

  tags:
    description: Tags associated with a post
    range: Tag
    multivalued: true

enums:
  PostStatus:
    description: Publication status for blog posts
    permissible_values:
      draft:
        description: Post is still being written
      published:
        description: Post is publicly visible
      archived:
        description: Post is no longer active
"#;

    // Parse the schema
    let mut parser = SchemaParser::new();
    let schema = parser.parse(schema_yaml)?;

    println!("Generating SQLAlchemy ORM models for blog schema...
");

    // Generate with default configuration (SQLAlchemy 2.0)
    let config = SQLAlchemyGeneratorConfig::default();
    let generator = SQLAlchemyGenerator::new(config);
    let output = generator.generate(&schema)?;

    println!("=== SQLAlchemy 2.0 Models (Default) ===
");
    println!("{}", output);

    // Generate with SQLAlchemy 1.4 configuration
    let config_v14 = SQLAlchemyGeneratorConfig {
        sqlalchemy_version: "1.4".to_string(),
        use_type_annotations: false,
        alembic_support: true,
        table_prefix: "blog_".to_string(),
        ..Default::default()
    };

    let generator_v14 = SQLAlchemyGenerator::new(config_v14);
    let output_v14 = generator_v14.generate(&schema)?;

    println!("

=== SQLAlchemy 1.4 Models with Alembic Support ===
");
    println!("{}", output_v14);

    // Generate minimal configuration without relationships
    let config_minimal = SQLAlchemyGeneratorConfig {
        generate_relationships: false,
        generate_indexes: false,
        generate_constraints: false,
        ..Default::default()
    };

    let generator_minimal = SQLAlchemyGenerator::new(config_minimal);
    let output_minimal = generator_minimal.generate(&schema)?;

    println!("

=== Minimal SQLAlchemy Models ===
");
    println!("{}", output_minimal);

    // Save to file
    std::fs::write("blog_models.py", &output)?;
    println!("

SQLAlchemy models saved to blog_models.py");

    Ok(())
}
