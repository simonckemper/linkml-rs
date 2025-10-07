//! Format-specific introspector implementations
//!
//! This module contains implementations of the DataIntrospector trait
//! for different data formats (XML, JSON, CSV, Excel/ODS).

pub mod csv;
pub mod excel;
pub mod json;
pub mod xml;

pub use csv::CsvIntrospector;
pub use excel::ExcelIntrospector;
pub use json::JsonIntrospector;
pub use xml::XmlIntrospector;
