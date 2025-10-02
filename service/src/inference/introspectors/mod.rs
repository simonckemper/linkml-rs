//! Format-specific introspector implementations
//!
//! This module contains implementations of the DataIntrospector trait
//! for different data formats (XML, JSON, CSV).

pub mod csv;
pub mod json;
pub mod xml;

pub use csv::CsvIntrospector;
pub use json::JsonIntrospector;
pub use xml::XmlIntrospector;
