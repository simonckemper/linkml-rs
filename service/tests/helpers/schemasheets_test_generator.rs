//! Helper functions to generate SchemaSheets format Excel files for testing

use rust_xlsxwriter::{Format, Workbook};
use std::path::Path;

/// Create a simple person schema in SchemaSheets format
pub fn create_person_schema_excel(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut workbook = Workbook::new();

    // Create main schema sheet
    let sheet = workbook.add_worksheet();
    sheet.set_name("Schema")?;

    // Add header row
    let header_format = Format::new().set_bold();
    sheet.write_with_format(0, 0, ">", &header_format)?;
    sheet.write_with_format(0, 1, "element_type", &header_format)?;
    sheet.write_with_format(0, 2, "field", &header_format)?;
    sheet.write_with_format(0, 3, "key", &header_format)?;
    sheet.write_with_format(0, 4, "multiplicity", &header_format)?;
    sheet.write_with_format(0, 5, "range", &header_format)?;
    sheet.write_with_format(0, 6, "desc", &header_format)?;
    sheet.write_with_format(0, 7, "is_a", &header_format)?;
    sheet.write_with_format(0, 8, "pattern", &header_format)?;
    sheet.write_with_format(0, 9, "schema.org:exactMatch", &header_format)?;
    sheet.write_with_format(0, 10, "skos:closeMatch", &header_format)?;

    // Person class
    let mut row = 1;
    sheet.write(row, 0, "Person")?;
    sheet.write(row, 1, "class")?;
    sheet.write(row, 6, "A person entity")?;
    sheet.write(row, 9, "schema:Person")?;
    sheet.write(row, 10, "foaf:Person")?;

    // Person.id
    row += 1;
    sheet.write(row, 2, "id")?;
    sheet.write(row, 3, "true")?;
    sheet.write(row, 4, "1")?;
    sheet.write(row, 5, "string")?;
    sheet.write(row, 6, "Unique identifier")?;

    // Person.name
    row += 1;
    sheet.write(row, 2, "name")?;
    sheet.write(row, 4, "1")?;
    sheet.write(row, 5, "string")?;
    sheet.write(row, 6, "Person's name")?;
    sheet.write(row, 9, "schema:name")?;

    // Person.email
    row += 1;
    sheet.write(row, 2, "email")?;
    sheet.write(row, 4, "0..1")?;
    sheet.write(row, 5, "string")?;
    sheet.write(row, 6, "Email address")?;
    sheet.write(row, 8, r"^[\w\.-]+@[\w\.-]+\.\w+$")?;
    sheet.write(row, 9, "schema:email")?;

    // Person.age
    row += 1;
    sheet.write(row, 2, "age")?;
    sheet.write(row, 4, "0..1")?;
    sheet.write(row, 5, "integer")?;
    sheet.write(row, 6, "Person's age")?;

    // Employee class (inherits from Person)
    row += 1;
    sheet.write(row, 0, "Employee")?;
    sheet.write(row, 1, "class")?;
    sheet.write(row, 6, "An employee")?;
    sheet.write(row, 7, "Person")?;

    // Employee.employee_id
    row += 1;
    sheet.write(row, 2, "employee_id")?;
    sheet.write(row, 3, "true")?;
    sheet.write(row, 4, "1")?;
    sheet.write(row, 5, "string")?;
    sheet.write(row, 6, "Employee ID")?;

    // Employee.department
    row += 1;
    sheet.write(row, 2, "department")?;
    sheet.write(row, 4, "1")?;
    sheet.write(row, 5, "string")?;
    sheet.write(row, 6, "Department name")?;

    // Status enum
    row += 1;
    sheet.write(row, 0, "Status")?;
    sheet.write(row, 1, "enum")?;
    sheet.write(row, 6, "Status values")?;

    // Status.ACTIVE
    row += 1;
    sheet.write(row, 2, "ACTIVE")?;
    sheet.write(row, 6, "Active status")?;

    // Status.INACTIVE
    row += 1;
    sheet.write(row, 2, "INACTIVE")?;
    sheet.write(row, 6, "Inactive status")?;

    // EmailType type
    row += 1;
    sheet.write(row, 0, "EmailType")?;
    sheet.write(row, 1, "type")?;
    sheet.write(row, 6, "Email address type")?;
    sheet.write(row, 7, "string")?;
    sheet.write(row, 8, r"^[\w\.-]+@[\w\.-]+\.\w+$")?;

    // required subset
    row += 1;
    sheet.write(row, 0, "required")?;
    sheet.write(row, 1, "subset")?;
    sheet.write(row, 6, "Required fields")?;

    // Create prefixes sheet
    let prefixes_sheet = workbook.add_worksheet();
    prefixes_sheet.set_name("prefixes")?;
    prefixes_sheet.write_with_format(0, 0, "prefix", &header_format)?;
    prefixes_sheet.write_with_format(0, 1, "uri", &header_format)?;
    prefixes_sheet.write(1, 0, "schema")?;
    prefixes_sheet.write(1, 1, "http://schema.org/")?;
    prefixes_sheet.write(2, 0, "foaf")?;
    prefixes_sheet.write(2, 1, "http://xmlns.com/foaf/0.1/")?;
    prefixes_sheet.write(3, 0, "skos")?;
    prefixes_sheet.write(3, 1, "http://www.w3.org/2004/02/skos/core#")?;

    // Create settings sheet
    let settings_sheet = workbook.add_worksheet();
    settings_sheet.set_name("settings")?;
    settings_sheet.write_with_format(0, 0, "setting", &header_format)?;
    settings_sheet.write_with_format(0, 1, "value", &header_format)?;
    settings_sheet.write(1, 0, "id")?;
    settings_sheet.write(1, 1, "https://example.org/person_schema")?;
    settings_sheet.write(2, 0, "name")?;
    settings_sheet.write(2, 1, "person_schema")?;
    settings_sheet.write(3, 0, "version")?;
    settings_sheet.write(3, 1, "1.0.0")?;
    settings_sheet.write(4, 0, "description")?;
    settings_sheet.write(4, 1, "A schema for person and employee entities")?;

    workbook.save(path)?;
    Ok(())
}

/// Create a minimal biolink schema in SchemaSheets format
pub fn create_biolink_minimal_excel(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut workbook = Workbook::new();

    // Create main schema sheet
    let sheet = workbook.add_worksheet();
    sheet.set_name("Schema")?;

    // Add header row
    let header_format = Format::new().set_bold();
    sheet.write_with_format(0, 0, ">", &header_format)?;
    sheet.write_with_format(0, 1, "element_type", &header_format)?;
    sheet.write_with_format(0, 2, "field", &header_format)?;
    sheet.write_with_format(0, 3, "multiplicity", &header_format)?;
    sheet.write_with_format(0, 4, "range", &header_format)?;
    sheet.write_with_format(0, 5, "desc", &header_format)?;
    sheet.write_with_format(0, 6, "is_a", &header_format)?;

    // NamedThing class
    let mut row = 1;
    sheet.write(row, 0, "NamedThing")?;
    sheet.write(row, 1, "class")?;
    sheet.write(row, 5, "A generic grouping for any identifiable entity")?;

    // NamedThing.id
    row += 1;
    sheet.write(row, 2, "id")?;
    sheet.write(row, 3, "1")?;
    sheet.write(row, 4, "string")?;
    sheet.write(row, 5, "A unique identifier")?;

    // NamedThing.name
    row += 1;
    sheet.write(row, 2, "name")?;
    sheet.write(row, 3, "0..1")?;
    sheet.write(row, 4, "string")?;
    sheet.write(row, 5, "A human-readable name")?;

    // BiologicalEntity class
    row += 1;
    sheet.write(row, 0, "BiologicalEntity")?;
    sheet.write(row, 1, "class")?;
    sheet.write(row, 5, "A biological entity")?;
    sheet.write(row, 6, "NamedThing")?;

    // Gene class
    row += 1;
    sheet.write(row, 0, "Gene")?;
    sheet.write(row, 1, "class")?;
    sheet.write(row, 5, "A region of DNA that codes for a product")?;
    sheet.write(row, 6, "BiologicalEntity")?;

    // Gene.symbol
    row += 1;
    sheet.write(row, 2, "symbol")?;
    sheet.write(row, 3, "0..1")?;
    sheet.write(row, 4, "string")?;
    sheet.write(row, 5, "Gene symbol")?;

    // Create prefixes sheet
    let prefixes_sheet = workbook.add_worksheet();
    prefixes_sheet.set_name("prefixes")?;
    prefixes_sheet.write_with_format(0, 0, "prefix", &header_format)?;
    prefixes_sheet.write_with_format(0, 1, "uri", &header_format)?;
    prefixes_sheet.write(1, 0, "biolink")?;
    prefixes_sheet.write(1, 1, "https://w3id.org/biolink/vocab/")?;

    // Create settings sheet
    let settings_sheet = workbook.add_worksheet();
    settings_sheet.set_name("settings")?;
    settings_sheet.write_with_format(0, 0, "setting", &header_format)?;
    settings_sheet.write_with_format(0, 1, "value", &header_format)?;
    settings_sheet.write(1, 0, "id")?;
    settings_sheet.write(1, 1, "https://w3id.org/biolink/biolink-model")?;
    settings_sheet.write(2, 0, "name")?;
    settings_sheet.write(2, 1, "biolink_minimal")?;

    workbook.save(path)?;
    Ok(())
}

/// Create an API models schema in SchemaSheets format with subsets
pub fn create_api_models_excel(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut workbook = Workbook::new();

    // Create main schema sheet
    let sheet = workbook.add_worksheet();
    sheet.set_name("Schema")?;

    // Add header row
    let header_format = Format::new().set_bold();
    sheet.write_with_format(0, 0, ">", &header_format)?;
    sheet.write_with_format(0, 1, "element_type", &header_format)?;
    sheet.write_with_format(0, 2, "field", &header_format)?;
    sheet.write_with_format(0, 3, "multiplicity", &header_format)?;
    sheet.write_with_format(0, 4, "range", &header_format)?;
    sheet.write_with_format(0, 5, "desc", &header_format)?;

    // ApiRequest class
    let mut row = 1;
    sheet.write(row, 0, "ApiRequest")?;
    sheet.write(row, 1, "class")?;
    sheet.write(row, 5, "An API request")?;

    // ApiRequest.method
    row += 1;
    sheet.write(row, 2, "method")?;
    sheet.write(row, 3, "1")?;
    sheet.write(row, 4, "string")?;
    sheet.write(row, 5, "HTTP method")?;

    // ApiRequest.path
    row += 1;
    sheet.write(row, 2, "path")?;
    sheet.write(row, 3, "1")?;
    sheet.write(row, 4, "string")?;
    sheet.write(row, 5, "Request path")?;

    // required subset
    row += 1;
    sheet.write(row, 0, "required")?;
    sheet.write(row, 1, "subset")?;
    sheet.write(row, 5, "Required fields")?;

    // public subset
    row += 1;
    sheet.write(row, 0, "public")?;
    sheet.write(row, 1, "subset")?;
    sheet.write(row, 5, "Public API fields")?;

    // internal subset
    row += 1;
    sheet.write(row, 0, "internal")?;
    sheet.write(row, 1, "subset")?;
    sheet.write(row, 5, "Internal fields")?;

    // Create settings sheet
    let settings_sheet = workbook.add_worksheet();
    settings_sheet.set_name("settings")?;
    settings_sheet.write_with_format(0, 0, "setting", &header_format)?;
    settings_sheet.write_with_format(0, 1, "value", &header_format)?;
    settings_sheet.write(1, 0, "id")?;
    settings_sheet.write(1, 1, "https://example.org/api_models")?;
    settings_sheet.write(2, 0, "name")?;
    settings_sheet.write(2, 1, "api_models")?;

    workbook.save(path)?;
    Ok(())
}

