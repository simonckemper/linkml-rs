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

/// Monitoring integration with performance metrics
pub mod monitoring_integration;

/// Command-line interface
pub mod cli;

/// Interactive validation mode
pub mod interactive;

/// REAL integrated service implementation (ARCHITECTURAL COMPLIANCE)
pub mod integrated_serve;

/// Migration tools
pub mod migration;

/// IDE integration support
pub mod ide;

/// Expression language for computed fields and dynamic validation
pub mod expression;

/// Rule engine for class-level validation
pub mod rule_engine;

/// SchemaView - High-level `API` for schema introspection
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

/// Enhanced CLI with all `LinkML` commands
pub mod cli_enhanced;

/// Plugin architecture for extensibility
pub mod plugin;

/// File system adapter for clean abstraction
pub mod file_system_adapter;

/// CLI file system adapter wrapper
pub mod cli_fs_adapter;

/// Configuration loading and management
pub mod config;

/// Configuration helper functions for loading and validation
pub mod config_helpers;

/// Namespace and CURIE resolution
pub mod namespace;

/// Inheritance resolution for classes and mixins
pub mod inheritance;

/// Utility functions and helpers
pub mod utils;

/// Prelude module for convenient imports
pub mod prelude;

/// Schema inference from data (data2linkmlschema)
pub mod inference;

// Re-export service trait and types
pub use factory::{create_linkml_service, create_linkml_service_with_config};
pub use linkml_core::error::LinkMLError;
pub use linkml_core::prelude::SchemaFormat;
pub use linkml_core::prelude::ValidationReport;
pub use linkml_core::prelude::*;
pub use service::LinkMLServiceImpl;

/// Test utilities for linkml service testing
#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils {
    use async_trait::async_trait;

    /// Mock LinkML service for testing
    pub struct MockLinkMLService {
        fail_on_load: bool,
        fail_on_validate: bool,
        custom_errors: Vec<linkml_core::ValidationError>,
        custom_warnings: Vec<linkml_core::ValidationWarning>,
    }

    impl MockLinkMLService {
        /// Creates a new mock LinkML service with default behavior for testing.
        ///
        /// The mock service starts in a "success" state where schema loading and
        /// validation operations will succeed with realistic mock data. Use the
        /// builder methods to configure specific failure scenarios for testing
        /// error handling paths in dependent services.
        ///
        /// # Examples
        ///
        /// ```rust
        /// # use linkml_service::test_utils::MockLinkMLService;
        /// let mock = MockLinkMLService::new();
        /// // Service will succeed on load_schema and validate operations
        /// ```
        pub fn new() -> Self {
            Self {
                fail_on_load: false,
                fail_on_validate: false,
                custom_errors: Vec::new(),
                custom_warnings: Vec::new(),
            }
        }

        /// Configures the mock to fail on schema loading operations.
        ///
        /// When enabled, calls to `load_schema` and `load_schema_str` will return
        /// a `ParseError` with a descriptive message. This enables testing of error
        /// handling in services that depend on schema loading, such as validation
        /// pipelines and data processing workflows.
        ///
        /// # Examples
        ///
        /// ```rust
        /// # use linkml_service::test_utils::MockLinkMLService;
        /// let mock = MockLinkMLService::new().fail_on_load();
        /// // All schema loading operations will now fail with ParseError
        /// ```
        pub fn fail_on_load(mut self) -> Self {
            self.fail_on_load = true;
            self
        }

        /// Configures the mock to fail on data validation operations.
        ///
        /// When enabled, calls to `validate` will return a `DataValidationError`
        /// instead of performing actual validation. This is essential for testing
        /// error recovery mechanisms in data ingestion pipelines and ensuring
        /// proper error propagation through the validation service chain.
        ///
        /// # Examples
        ///
        /// ```rust
        /// # use linkml_service::test_utils::MockLinkMLService;
        /// let mock = MockLinkMLService::new().fail_on_validate();
        /// // All validation operations will now fail with DataValidationError
        /// ```
        pub fn fail_on_validate(mut self) -> Self {
            self.fail_on_validate = true;
            self
        }

        /// Configures the mock to return specific validation errors and warnings.
        ///
        /// This method enables precise testing of validation result handling by
        /// allowing tests to inject specific error and warning conditions. The
        /// validation will succeed (not fail) but return the specified errors
        /// and warnings in the ValidationReport, enabling testing of partial
        /// validation scenarios and warning aggregation logic.
        ///
        /// # Arguments
        ///
        /// * `errors` - Validation errors to include in the report. If non-empty,
        ///   the validation report will be marked as invalid.
        /// * `warnings` - Validation warnings to include in the report. These do
        ///   not affect the validity status but enable testing warning handling.
        ///
        /// # Examples
        ///
        /// ```rust
        /// # use linkml_service::test_utils::MockLinkMLService;
        /// # use linkml_core::{ValidationError, ValidationWarning};
        /// let errors = vec![ValidationError::missing_required_field("name")];
        /// let warnings = vec![ValidationWarning::deprecated_field("old_field")];
        /// let mock = MockLinkMLService::new().with_results(errors, warnings);
        /// // Validation will return these specific errors and warnings
        /// ```
        pub fn with_results(
            mut self,
            errors: Vec<linkml_core::ValidationError>,
            warnings: Vec<linkml_core::ValidationWarning>,
        ) -> Self {
            self.custom_errors = errors;
            self.custom_warnings = warnings;
            self
        }
    }

    #[async_trait]
    impl linkml_core::LinkMLService for MockLinkMLService {
        async fn load_schema(
            &self,
            _path: &std::path::Path,
        ) -> linkml_core::Result<linkml_core::SchemaDefinition> {
            if self.fail_on_load {
                return Err(linkml_core::error::LinkMLError::ParseError {
                    message: format!("Mock load failure for path: {}", _path.display()),
                    location: Some(_path.to_string_lossy().to_string()),
                });
            }

            use indexmap::IndexMap;

            Ok(linkml_core::SchemaDefinition {
                id: "mock-schema".to_string(),
                name: "MockSchema".to_string(),
                title: Some("Mock Schema".to_string()),
                description: Some("Mock schema for testing".to_string()),
                version: Some("1.0.0".to_string()),
                license: None,
                default_prefix: Some("mock".to_string()),
                prefixes: IndexMap::new(),
                imports: vec![],
                classes: IndexMap::new(),
                slots: IndexMap::new(),
                types: IndexMap::new(),
                enums: IndexMap::new(),
                subsets: IndexMap::new(),
                default_range: None,
                generation_date: None,
                source_file: None,
                metamodel_version: None,
                settings: None,
                annotations: None,
                contributors: vec![],
                status: None,
                categories: vec![],
                keywords: vec![],
                see_also: vec![],
            })
        }

        async fn load_schema_str(
            &self,
            _content: &str,
            _format: linkml_core::SchemaFormat,
        ) -> linkml_core::Result<linkml_core::SchemaDefinition> {
            self.load_schema(std::path::Path::new("mock.yaml")).await
        }

        async fn validate(
            &self,
            _data: &serde_json::Value,
            _schema: &linkml_core::SchemaDefinition,
            _target_class: &str,
        ) -> linkml_core::Result<linkml_core::ValidationReport> {
            if self.fail_on_validate {
                return Err(linkml_core::error::LinkMLError::DataValidationError {
                    message: "Mock validation failure".to_string(),
                    path: None,
                    expected: None,
                    actual: None,
                });
            }

            let has_errors = !self.custom_errors.is_empty();

            Ok(linkml_core::ValidationReport {
                valid: !has_errors,
                errors: self.custom_errors.clone(),
                warnings: self.custom_warnings.clone(),
                timestamp: Some(chrono::Utc::now()),
                schema_id: Some("mock-schema".to_string()),
            })
        }
    }
}
