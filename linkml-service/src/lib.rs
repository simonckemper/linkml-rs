//! `LinkML` Service Implementation
//!
//! This crate provides the core `LinkML` validation service for `RootReal`,
//! implementing 100% feature parity with Python `LinkML` plus native enhancements.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

/// Service factory and initialization
pub mod factory;

/// Factory v2 with Configuration Service integration
pub mod factory_v2;

/// Factory v3 with DBMS Service integration
pub mod factory_v3;

/// Service implementation
pub mod service;

/// Schema parsing
pub mod parser;

/// Data validation
pub mod validator;

/// Code generation
pub mod generator;

/// Pattern matching with named captures
pub mod pattern;

/// Instance-based validation
pub mod instance;

/// Schema transformation
pub mod transform;

/// RootReal service integration
pub mod integration;

/// Command-line interface
pub mod cli;

/// Interactive validation mode
pub mod interactive;

/// Migration tools
pub mod migration;

/// IDE integration support
pub mod ide;

/// Expression language for computed fields and dynamic validation
pub mod expression;

/// Rule engine for class-level validation
pub mod rule_engine;

/// SchemaView - High-level API for schema introspection
pub mod schema_view;

/// Performance optimization utilities
pub mod performance;

/// Security utilities and input validation
pub mod security;

/// Data loading and dumping functionality
pub mod loader;

/// Array support for N-dimensional data
pub mod array;

/// Schema manipulation utilities (diff, merge, lint)
pub mod schema;

/// Enhanced CLI with all LinkML commands
pub mod cli_enhanced;

/// Plugin architecture for extensibility
pub mod plugin;

// Re-export service trait and types
pub use factory::{create_linkml_service, create_linkml_service_with_config};
pub use linkml_core::prelude::*;
pub use service::LinkMLServiceImpl;
