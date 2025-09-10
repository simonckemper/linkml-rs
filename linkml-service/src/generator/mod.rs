//! Code generation for LinkML schemas
//!
//! This module provides comprehensive code generation from LinkML schemas
//! supporting multiple target languages and formats.

// Core generator infrastructure
pub mod base;
pub mod options;
pub mod traits;
pub mod registry;
pub mod plugin;
pub mod namespace_manager;

// Rust generator modules (refactored)
pub mod core;
pub mod classes;
pub mod fields;
pub mod builders;
pub mod validation;
pub mod implementations;
pub mod rust_traits;

// Language-specific generators
pub mod rust_generator;
pub mod typeql_generator;
pub mod typeql_generator_enhanced;
pub mod csv;
pub mod excel;
pub mod golang;
pub mod graphql_generator;
pub mod graphviz;
pub mod html;
pub mod java;
pub mod javascript;
pub mod jsonld_context;
pub mod json_ld;
pub mod json_schema;
pub mod markdown;
pub mod mermaid;
pub mod openapi;
pub mod plantuml;
pub mod prefix_map;
pub mod protobuf;
pub mod pydantic;
pub mod python_dataclass;
pub mod rdf;
pub mod sqlalchemy;
pub mod sql;
pub mod shex;
pub mod shacl;
pub mod sparql;
pub mod sssom;
pub mod summary;
pub mod typescript;
pub mod yaml_validator;
pub mod yuml;
pub mod typeql_constraints;
pub mod typeql_relation_analyzer;
pub mod typeql_rule_generator;
pub mod typeql_expression_translator;
pub mod typeql_role_inheritance;
pub mod array_support;
pub mod doc;
pub mod yaml;

// Re-export main types
pub use core::RustGenerator;
pub use traits::{AsyncGenerator, CodeFormatter, GeneratedOutput, Generator, GeneratorError, GeneratorResult, GeneratorConfig};
pub use options::{GeneratorOptions, IndentStyle, OutputFormat};
pub use registry::{GeneratorInfo, GeneratorRegistry};

// Re-export generators
pub use csv::CsvGenerator;
pub use excel::ExcelGenerator;
pub use golang::GoGenerator;
pub use graphql_generator::GraphQLGenerator;
pub use graphviz::GraphvizGenerator;
pub use html::HtmlGenerator;
pub use java::JavaGenerator;
pub use javascript::JavaScriptGenerator;
pub use jsonld_context::{JsonLdContextGenerator, JsonLdContextGeneratorConfig};
pub use json_ld::JsonLdGenerator;
pub use json_schema::JsonSchemaGenerator;
pub use markdown::MarkdownGenerator;
pub use mermaid::{MermaidGenerator, MermaidDiagramType};
pub use namespace_manager::{NamespaceManagerGenerator, NamespaceManagerGeneratorConfig};
pub use openapi::OpenApiGenerator;
pub use plantuml::PlantUmlGenerator;
pub use prefix_map::{PrefixMapGenerator, PrefixMapGeneratorConfig, PrefixMapFormat};
pub use protobuf::ProtobufGenerator;
pub use pydantic::PydanticGenerator;
pub use python_dataclass::PythonDataclassGenerator;
pub use rdf::RdfGenerator;
pub use sqlalchemy::{SQLAlchemyGenerator, SQLAlchemyGeneratorConfig};
pub use sql::SQLGenerator;
pub use shex::ShExGenerator;
pub use shacl::ShaclGenerator;
pub use sparql::SparqlGenerator;
pub use sssom::{SssomGenerator, SssomGeneratorConfig, SssomFormat};
pub use summary::{SummaryGenerator, SummaryGeneratorConfig, SummaryFormat};
pub use typescript::TypeScriptGenerator;
pub use namespace_manager::TargetLanguage;
pub use yaml_validator::{YamlValidatorGenerator, YamlValidatorGeneratorConfig, ValidationFramework};
pub use typeql_generator::TypeQLGenerator;
pub use yuml::YumlGenerator;
