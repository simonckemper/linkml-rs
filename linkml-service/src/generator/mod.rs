//! Code generation for LinkML schemas
//!
//! This module provides comprehensive code generation from LinkML schemas
//! supporting multiple target languages and formats.

// Core generator infrastructure
pub mod base;
pub mod namespace_manager;
pub mod options;
pub mod plugin;
pub mod registry;
pub mod traits;

// Rust generator modules (refactored)
pub mod builders;
pub mod classes;
pub mod core;
pub mod fields;
pub mod implementations;
pub mod rust_traits;
pub mod validation;

// Language-specific generators
pub mod array_support;
pub mod csv;
pub mod doc;
pub mod excel;
pub mod golang;
pub mod graphql_generator;
pub mod graphviz;
pub mod html;
pub mod java;
pub mod javascript;
pub mod json_ld;
pub mod json_schema;
pub mod jsonld_context;
pub mod markdown;
pub mod mermaid;
pub mod openapi;
pub mod plantuml;
pub mod prefix_map;
pub mod protobuf;
pub mod pydantic;
pub mod python_dataclass;
pub mod rdf;
pub mod rust_generator;
pub mod shacl;
pub mod shex;
pub mod sparql;
pub mod sql;
pub mod sqlalchemy;
pub mod sssom;
pub mod summary;
pub mod typeql_constraints;
pub mod typeql_expression_translator;
pub mod typeql_generator;
pub mod typeql_generator_enhanced;
pub mod typeql_relation_analyzer;
pub mod typeql_role_inheritance;
pub mod typeql_rule_generator;
pub mod typescript;
pub mod yaml;
pub mod yaml_validator;
pub mod yuml;

// Re-export main types
pub use core::RustGenerator;
pub use options::{GeneratorOptions, IndentStyle, OutputFormat};
pub use registry::{GeneratorInfo, GeneratorRegistry};
pub use traits::{
    AsyncGenerator, CodeFormatter, GeneratedOutput, Generator, GeneratorConfig, GeneratorError,
    GeneratorResult,
};

// Re-export generators
pub use csv::CsvGenerator;
pub use excel::ExcelGenerator;
pub use golang::GoGenerator;
pub use graphql_generator::GraphQLGenerator;
pub use graphviz::GraphvizGenerator;
pub use html::HtmlGenerator;
pub use java::JavaGenerator;
pub use javascript::JavaScriptGenerator;
pub use json_ld::JsonLdGenerator;
pub use json_schema::JsonSchemaGenerator;
pub use jsonld_context::{JsonLdContextGenerator, JsonLdContextGeneratorConfig};
pub use markdown::MarkdownGenerator;
pub use mermaid::{MermaidDiagramType, MermaidGenerator};
pub use namespace_manager::TargetLanguage;
pub use namespace_manager::{NamespaceManagerGenerator, NamespaceManagerGeneratorConfig};
pub use openapi::OpenApiGenerator;
pub use plantuml::PlantUmlGenerator;
pub use prefix_map::{PrefixMapFormat, PrefixMapGenerator, PrefixMapGeneratorConfig};
pub use protobuf::ProtobufGenerator;
pub use pydantic::PydanticGenerator;
pub use python_dataclass::PythonDataclassGenerator;
pub use rdf::RdfGenerator;
pub use shacl::ShaclGenerator;
pub use shex::ShExGenerator;
pub use sparql::SparqlGenerator;
pub use sql::SQLGenerator;
pub use sqlalchemy::{SQLAlchemyGenerator, SQLAlchemyGeneratorConfig};
pub use sssom::{SssomFormat, SssomGenerator, SssomGeneratorConfig};
pub use summary::{SummaryFormat, SummaryGenerator, SummaryGeneratorConfig};
pub use typeql_generator::TypeQLGenerator;
pub use typescript::TypeScriptGenerator;
pub use yaml_validator::{
    ValidationFramework, YamlValidatorGenerator, YamlValidatorGeneratorConfig,
};
pub use yuml::YumlGenerator;
