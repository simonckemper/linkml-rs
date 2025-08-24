//! Code generation module for LinkML service

pub mod array_support;
pub mod base;
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
pub mod namespace_manager;
pub mod openapi;
pub mod options;
pub mod owl_rdf;
pub mod plantuml;
pub mod plugin;
pub mod prefix_map;
pub mod protobuf;
pub mod pydantic;
pub mod python_dataclass;
pub mod registry;
pub mod rust_generator;
pub mod shacl;
pub mod shex;
pub mod sparql;
pub mod sql;
pub mod sqlalchemy;
pub mod sssom;
pub mod summary;
pub mod traits;
pub mod typeql_constraints;
pub mod typeql_generator;
pub mod typeql_generator_enhanced;
pub mod typeql_relation_analyzer;
pub mod typeql_role_inheritance;
pub mod typescript;
pub mod yaml_validator;
pub mod yuml;
// pub mod project; // Temporarily disabled due to compilation issues
pub mod rdf;
pub mod traits_v2;
pub mod typeql_expression_translator;
pub mod typeql_rule_generator;
pub mod yaml;

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
pub use mermaid::{MermaidDiagramType, MermaidGenerator, MermaidOptions};
pub use namespace_manager::{
    NamespaceManagerGenerator, NamespaceManagerGeneratorConfig, TargetLanguage,
};
pub use openapi::OpenApiGenerator;
pub use options::{GeneratorOptions, IndentStyle, OutputFormat};
pub use owl_rdf::{OwlRdfGenerator, RdfFormat, RdfGenerator, RdfMode};
pub use plantuml::{PlantUmlDiagramType, PlantUmlGenerator};
pub use plugin::{GeneratorPlugin, PluginManager};
pub use prefix_map::{PrefixMapFormat, PrefixMapGenerator, PrefixMapGeneratorConfig};
pub use protobuf::ProtobufGenerator;
pub use pydantic::PydanticGenerator;
pub use python_dataclass::PythonDataclassGenerator;
pub use registry::{GeneratorInfo, GeneratorRegistry};
pub use rust_generator::RustGenerator;
pub use shacl::ShaclGenerator;
pub use shex::{ShExGenerator, ShExStyle};
pub use sparql::{SparqlGenerator, SparqlQueryType};
pub use sql::SQLGenerator;
pub use sqlalchemy::{SQLAlchemyGenerator, SQLAlchemyGeneratorConfig};
pub use sssom::{SssomFormat, SssomGenerator, SssomGeneratorConfig};
pub use summary::{SummaryFormat, SummaryGenerator, SummaryGeneratorConfig};
pub use traits::{
    AsyncGenerator, CodeFormatter, GeneratedOutput, Generator, GeneratorError, GeneratorResult,
};
pub use typeql_generator::TypeQLGenerator;
pub use typeql_generator_enhanced::EnhancedTypeQLGenerator;
pub use typescript::TypeScriptGenerator;
pub use yaml_validator::{
    ValidationFramework, YamlValidatorGenerator, YamlValidatorGeneratorConfig,
};
pub use yuml::{YumlDiagramType, YumlGenerator};
// pub use project::{ProjectGenerator, ProjectGeneratorConfig, ProjectTarget, LicenseType};
pub use rdf::RdfGenerator as PlainRdfGenerator;
pub use yaml::YamlGenerator;
