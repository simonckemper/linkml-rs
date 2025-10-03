//! Schema generation API demonstration for LinkML service
//!
//! This example demonstrates code generation capabilities:
//! - TypeQL schema generation for TypeDB
//! - SQL DDL generation
//! - GraphQL schema generation
//! - Rust struct generation
//! - Documentation generation
//!
//! NOTE: This demonstrates the API. In production, initialize
//! the service with RootReal dependencies.

fn main() {
    println!("LinkML Schema Generation API Demonstration");
    println!("=========================================
");

    // Show the schema we'll use for generation
    demonstrate_source_schema();

    // Show TypeQL generation
    demonstrate_typeql_generation();

    // Show SQL generation
    demonstrate_sql_generation();

    // Show GraphQL generation
    demonstrate_graphql_generation();

    // Show Rust generation
    demonstrate_rust_generation();

    // Show documentation generation
    demonstrate_doc_generation();
}

fn demonstrate_source_schema() {
    println!("1. Source Schema for Generation:
");

    let schema = r#"
id: https://example.org/library-schema
name: LibrarySchema
description: Schema for a library management system

classes:
  Book:
    description: A physical or digital book
    slots:
      - isbn
      - title
      - authors
      - publication_year
      - genres

  Author:
    description: Book author
    slots:
      - author_id
      - name
      - birth_year
      - biography

  Member:
    description: Library member
    slots:
      - member_id
      - name
      - email
      - join_date
      - borrowed_books

  Loan:
    description: Book loan record
    slots:
      - loan_id
      - member
      - book
      - loan_date
      - due_date
      - return_date

slots:
  isbn:
    identifier: true
    range: string
    pattern: "^(97[89])?\\d{10}$"

  title:
    range: string
    required: true

  authors:
    range: Author
    multivalued: true
    minimum_cardinality: 1

  # ... more slots defined ...
"#;

    println!("{}", schema);
    println!("
This schema will be transformed into various formats...
");
}

fn demonstrate_typeql_generation() {
    println!("2. TypeQL Generation (for TypeDB):
");

    let expected_typeql = r#"
define

# Entities
book sub entity,
    owns isbn @key,
    owns title,
    owns publication_year,
    plays authorship:written_work,
    plays loan:borrowed_item;

author sub entity,
    owns author_id @key,
    owns name,
    owns birth_year,
    owns biography,
    plays authorship:writer;

member sub entity,
    owns member_id @key,
    owns name,
    owns email,
    owns join_date,
    plays loan:borrower;

loan sub entity,
    owns loan_id @key,
    owns loan_date,
    owns due_date,
    owns return_date,
    plays loan:loan_record;

# Relations
authorship sub relation,
    relates writer,
    relates written_work;

loan sub relation,
    relates borrower,
    relates borrowed_item,
    relates loan_record;

# Attributes
isbn sub attribute, value string, regex "^(97[89])?\\d{10}$";
title sub attribute, value string;
publication_year sub attribute, value long;
author_id sub attribute, value string;
name sub attribute, value string;
# ... more attributes ...
"#;

    println!("Generated TypeQL schema:");
    println!("{}", expected_typeql);

    println!("
Usage in production:");
    println!(
        r#"
let typeql = linkml_service.generate_typeql(&schema).await?;
typedb_client.define_schema(&typeql).await?;
"#
    );
}

fn demonstrate_sql_generation() {
    println!("
3. SQL DDL Generation:
");

    let expected_sql = r#"
-- Generated SQL DDL for PostgreSQL

CREATE TABLE book (
    isbn VARCHAR(13) PRIMARY KEY CHECK (isbn ~ '^(97[89])?\d{10}$'),
    title TEXT NOT NULL,
    publication_year INTEGER
);

CREATE TABLE author (
    author_id VARCHAR(255) PRIMARY KEY,
    name TEXT NOT NULL,
    birth_year INTEGER,
    biography TEXT
);

CREATE TABLE member (
    member_id VARCHAR(255) PRIMARY KEY,
    name TEXT NOT NULL,
    email VARCHAR(255) NOT NULL,
    join_date DATE NOT NULL
);

CREATE TABLE loan (
    loan_id VARCHAR(255) PRIMARY KEY,
    member_id VARCHAR(255) NOT NULL REFERENCES member(member_id),
    isbn VARCHAR(13) NOT NULL REFERENCES book(isbn),
    loan_date TIMESTAMP NOT NULL,
    due_date TIMESTAMP NOT NULL,
    return_date TIMESTAMP
);

-- Junction table for many-to-many relationship
CREATE TABLE book_author (
    isbn VARCHAR(13) REFERENCES book(isbn),
    author_id VARCHAR(255) REFERENCES author(author_id),
    PRIMARY KEY (isbn, author_id)
);

-- Indexes for performance
CREATE INDEX idx_loan_member ON loan(member_id);
CREATE INDEX idx_loan_book ON loan(isbn);
CREATE INDEX idx_loan_dates ON loan(loan_date, due_date);
"#;

    println!("Generated SQL DDL:");
    println!("{}", expected_sql);

    println!("
Dialect options:");
    println!("- PostgreSQL (default)");
    println!("- MySQL");
    println!("- SQLite");
    println!("- SQL Server");
}

fn demonstrate_graphql_generation() {
    println!("
4. GraphQL Schema Generation:
");

    let expected_graphql = r#"
# Generated GraphQL Schema

type Book {
  isbn: ID!
  title: String!
  authors: [Author!]!
  publicationYear: Int
  genres: [String!]
  loans: [Loan!]
}

type Author {
  authorId: ID!
  name: String!
  birthYear: Int
  biography: String
  books: [Book!]
}

type Member {
  memberId: ID!
  name: String!
  email: String!
  joinDate: Date!
  borrowedBooks: [Loan!]
}

type Loan {
  loanId: ID!
  member: Member!
  book: Book!
  loanDate: DateTime!
  dueDate: DateTime!
  returnDate: DateTime
}

# Input types for mutations
input BookInput {
  isbn: String!
  title: String!
  authorIds: [ID!]!
  publicationYear: Int
  genres: [String!]
}

input MemberInput {
  name: String!
  email: String!
}

# Root types
type Query {
  book(isbn: ID!): Book
  books(limit: Int, offset: Int): [Book!]!
  author(authorId: ID!): Author
  authors(limit: Int, offset: Int): [Author!]!
  member(memberId: ID!): Member
  members(limit: Int, offset: Int): [Member!]!
  activeLoans: [Loan!]!
  overdueLoans: [Loan!]!
}

type Mutation {
  createBook(input: BookInput!): Book!
  createMember(input: MemberInput!): Member!
  createLoan(memberId: ID!, isbn: ID!): Loan!
  returnBook(loanId: ID!): Loan!
}
"#;

    println!("Generated GraphQL schema:");
    println!("{}", expected_graphql);
}

fn demonstrate_rust_generation() {
    println!("
5. Rust Code Generation:
");

    let expected_rust = r#"
// Generated Rust code with serde support

use serde::{Deserialize, Serialize};
use chrono::{DateTime, NaiveDate, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Book {
    #[serde(rename = "isbn")]
    pub isbn: String,

    #[serde(rename = "title")]
    pub title: String,

    #[serde(rename = "authors")]
    pub authors: Vec<Author>,

    #[serde(rename = "publication_year", skip_serializing_if = "Option::is_none")]
    pub publication_year: Option<i32>,

    #[serde(rename = "genres", default, skip_serializing_if = "Vec::is_empty")]
    pub genres: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    #[serde(rename = "author_id")]
    pub author_id: String,

    #[serde(rename = "name")]
    pub name: String,

    #[serde(rename = "birth_year", skip_serializing_if = "Option::is_none")]
    pub birth_year: Option<i32>,

    #[serde(rename = "biography", skip_serializing_if = "Option::is_none")]
    pub biography: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    #[serde(rename = "member_id")]
    pub member_id: String,

    #[serde(rename = "name")]
    pub name: String,

    #[serde(rename = "email")]
    pub email: String,

    #[serde(rename = "join_date")]
    pub join_date: NaiveDate,

    #[serde(rename = "borrowed_books", default, skip_serializing_if = "Vec::is_empty")]
    pub borrowed_books: Vec<Loan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Loan {
    #[serde(rename = "loan_id")]
    pub loan_id: String,

    #[serde(rename = "member")]
    pub member: Box<Member>,

    #[serde(rename = "book")]
    pub book: Box<Book>,

    #[serde(rename = "loan_date")]
    pub loan_date: DateTime<Utc>,

    #[serde(rename = "due_date")]
    pub due_date: DateTime<Utc>,

    #[serde(rename = "return_date", skip_serializing_if = "Option::is_none")]
    pub return_date: Option<DateTime<Utc>>,
}

// Validation implementations
impl Book {
    pub fn validate(&self) -> std::result::Result<(), ValidationError> {
        // Validate ISBN pattern
        let isbn_regex = regex::Regex::new(r"^(97[89])?\d{10}$")?;
        if !isbn_regex.is_match(&self.isbn) {
            return Err(ValidationError::PatternMismatch {
                field: "isbn".to_string(),
                pattern: "^(97[89])?\\d{10}$".to_string(),
                value: self.isbn.clone(),
            });
        }

        // Validate required fields
        if self.title.is_empty() {
            return Err(ValidationError::RequiredField {
                field: "title".to_string(),
            });
        }

        // Validate minimum cardinality
        if self.authors.is_empty() {
            return Err(ValidationError::MinCardinality {
                field: "authors".to_string(),
                min: 1,
                actual: 0,
            });
        }

        Ok(())
    }
}
"#;

    println!("Generated Rust code:");
    println!("{}", expected_rust);

    println!("
Features:");
    println!("- Serde serialization support");
    println!("- Validation methods");
    println!("- Builder pattern (optional)");
    println!("- Async trait implementations (optional)");
}

fn demonstrate_doc_generation() {
    println!("
6. Documentation Generation:
");

    println!("Supported formats:");
    println!("- Markdown (for GitHub/GitLab)");
    println!("- HTML (for web documentation)");
    println!("- ReStructuredText (for Sphinx)");
    println!("- JSON-LD context");

    let example_markdown = r#"
# LibrarySchema Documentation

## Overview
Schema for a library management system

## Classes

### Book
A physical or digital book

**Slots:**
- **isbn** (string) - *identifier* - International Standard Book Number
  - Pattern: `^(97[89])?\d{10}$`
- **title** (string) - *required* - Book title
- **authors** ([Author]) - *required* - Book authors (minimum: 1)
- **publication_year** (integer) - Year of publication
- **genres** ([string]) - Literary genres

### Author
Book author

**Slots:**
- **author_id** (string) - *identifier* - Unique author identifier
- **name** (string) - *required* - Author's full name
- **birth_year** (integer) - Year of birth
- **biography** (string) - Author biography

## Relationships

```mermaid
graph LR
    Book -->|has| Author
    Member -->|borrows| Book
    Loan -->|references| Book
    Loan -->|references| Member
```
"#;

    println!("Example Markdown documentation:");
    println!("{}", example_markdown);
}

/// Show generator configuration options
fn _generator_options() {
    println!("
7. Generator Options:
");

    println!(
        r#"
// Configure generation options
let options = GeneratorOptions {{
    // Common options
    include_comments: true,
    include_examples: true,

    // Language-specific options
    rust_options: RustOptions {{
        derive_traits: vec!["Debug", "Clone", "PartialEq"],
        use_builders: true,
        async_support: true,
    }},

    sql_options: SqlOptions {{
        dialect: SqlDialect::PostgreSQL,
        include_indexes: true,
        include_constraints: true,
    }},

    graphql_options: GraphQLOptions {{
        include_mutations: true,
        include_subscriptions: false,
        relay_support: true,
    }},
}};

// Generate with options
let rust_code = linkml_service.generate_rust(&schema, &options).await?;
"#
    );
}
