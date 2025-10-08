//! SchemaSheets format support for LinkML
//!
//! This module provides support for the SchemaSheets format, which is a
//! standardized Excel/Google Sheets format for representing LinkML schemas.
//!
//! SchemaSheets uses specific column headers and formatting conventions to
//! encode schema metadata in spreadsheets, enabling lossless roundtrip
//! conversion between LinkML YAML and Excel/Google Sheets.
//!
//! ## Format Specification
//!
//! SchemaSheets uses the following column headers:
//! - `>` prefix: Indicates a class definition row
//! - `field`: Attribute/slot name
//! - `key`: Identifier designation (true/false)
//! - `multiplicity`: Cardinality (e.g., "0..1", "1", "0..*", "1..*")
//! - `range`: Data type or class reference
//! - `desc`: Description/documentation
//! - `is_a`: Parent class for inheritance
//! - `mixin`: Mixin class
//! - `required`: Whether field is required (true/false)
//! - `pattern`: Regex pattern for validation
//! - `minimum_value`: Minimum numeric value
//! - `maximum_value`: Maximum numeric value
//! - Mapping columns: External vocabulary mappings (e.g., "schema.org", "skos:exactMatch")
//!
//! ## Example SchemaSheets Format
//!
//! ```text
//! | >         | field      | key   | multiplicity | range   | desc                    | is_a        |
//! |-----------|------------|-------|--------------|---------|-------------------------|-------------|
//! | Person    |            |       |              |         | A person entity         |             |
//! |           | id         | true  | 1            | string  | Unique identifier       |             |
//! |           | name       | false | 1            | string  | Person's name           |             |
//! |           | age        | false | 0..1         | integer | Person's age            |             |
//! |           | email      | false | 0..*         | string  | Email addresses         |             |
//! | Employee  |            |       |              |         | An employee             | Person      |
//! |           | employee_id| true  | 1            | string  | Employee ID             |             |
//! |           | department | false | 1            | string  | Department name         |             |
//! ```
//!
//! ## Modules
//!
//! - `parser`: Parse SchemaSheets format Excel files into LinkML schemas
//! - `generator`: Generate SchemaSheets format Excel files from LinkML schemas
//! - `types`: Common types and utilities for SchemaSheets processing

pub mod config;
pub mod generator;
pub mod parser;
pub mod types;

pub use config::SchemaSheetsConfig;
pub use generator::SchemaSheetsGenerator;
pub use parser::SchemaSheetsParser;
pub use types::{SchemaSheetRow, SchemaSheetType};

