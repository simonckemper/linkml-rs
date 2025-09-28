//! Optimized generator traits using Arc<SchemaDefinition>
//!
//! This module provides generator traits that use Arc for efficient
//! schema sharing across generator implementations.

use async_trait::async_trait;
use std::sync::Arc;

use linkml_core::{error::Result, schema_arc::ArcSchema};

/// Options for code generation
#[derive(Debug, Clone, Default)]
pub struct GeneratorOptions {
    /// Target directory for generated files
    pub output_dir: Option<String>,
    /// Whether to generate documentation
    pub include_docs: bool,
    /// Whether to validate schema before generation
    pub validate_first: bool,
    /// Custom template directory
    pub template_dir: Option<String>,
    /// Additional generator-specific options
    pub extra: std::collections::HashMap<String, String>}

/// Result of code generation
#[derive(Debug)]
pub struct GenerationResult {
    /// Generated file paths
    pub files: Vec<String>,
    /// Main output content (for single-file generators)
    pub content: Option<String>,
    /// Any warnings produced
    pub warnings: Vec<String>,
    /// Generation statistics
    pub stats: GenerationStats}

/// Statistics about the generation process
#[derive(Debug, Default)]
pub struct GenerationStats {
    /// Number of classes generated
    pub classes_generated: usize,
    /// Number of slots generated
    pub slots_generated: usize,
    /// Number of types generated
    pub types_generated: usize,
    /// Number of enums generated
    pub enums_generated: usize,
    /// Number of files created
    pub files_created: usize,
    /// Number of bytes written
    pub bytes_written: usize}

/// Optimized code generator trait using Arc
#[async_trait]
pub trait CodeGeneratorV2: Send + Sync {
    /// Generate code from schema
    async fn generate(
        &self,
        schema: ArcSchema,
        options: GeneratorOptions,
    ) -> Result<GenerationResult>;

    /// Get the name of this generator
    fn name(&self) -> &'static str;

    /// Get supported file extensions
    fn file_extensions(&self) -> Vec<&'static str>;

    /// Check if generator supports a particular feature
    fn supports_feature(&self, _feature: &str) -> bool {
        false
    }
}

/// Generator that can work incrementally
#[async_trait]
pub trait IncrementalGenerator: CodeGeneratorV2 {
    /// Generate only for specific classes
    async fn generate_classes(
        &self,
        schema: ArcSchema,
        class_names: &[String],
        options: GeneratorOptions,
    ) -> Result<GenerationResult>;

    /// Generate only for changes between schemas
    async fn generate_diff(
        &self,
        old_schema: ArcSchema,
        new_schema: ArcSchema,
        options: GeneratorOptions,
    ) -> Result<GenerationResult>;
}

/// Base implementation helper for generators
pub struct GeneratorBase {
    _name: &'static str,
    _extensions: Vec<&'static str>}

impl GeneratorBase {
    /// Create a new generator base with name and supported extensions
    pub fn new(name: &'static str, extensions: Vec<&'static str>) -> Self {
        Self {
            _name: name,
            _extensions: extensions}
    }
}

/// Registry for generators with Arc schema support
pub struct GeneratorRegistry {
    generators: dashmap::DashMap<String, Arc<dyn CodeGeneratorV2>>}

impl GeneratorRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            generators: dashmap::DashMap::new()}
    }

    /// Register a generator
    pub fn register<G: CodeGeneratorV2 + 'static>(&self, generator: G) {
        let name = generator.name().to_string();
        self.generators.insert(name, Arc::new(generator));
    }

    /// Get a generator by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn CodeGeneratorV2>> {
        self.generators.get(name).map(|g| Arc::clone(&g))
    }

    /// List all registered generators
    pub fn list(&self) -> Vec<String> {
        self.generators.iter().map(|e| e.key().clone()).collect()
    }
}

/// Macro to help implement generators with Arc support
#[macro_export]
macro_rules! impl_generator_v2 {
    ($name:ident, $generator_name:expr, $extensions:expr) => {
        #[async_trait]
        impl CodeGeneratorV2 for $name {
            async fn generate(
                &self,
                schema: ArcSchema,
                options: GeneratorOptions,
            ) -> Result<GenerationResult> {
                self.generate_impl(schema, options).await
            }

            fn name(&self) -> &'static str {
                $generator_name
            }

            fn file_extensions(&self) -> Vec<&'static str> {
                $extensions
            }
        }
    };
}

/// Example of a generator implementation using Arc
pub struct ExampleGeneratorV2 {
    _base: GeneratorBase}

impl ExampleGeneratorV2 {
    /// Create a new example generator
    pub fn new() -> Self {
        Self {
            _base: GeneratorBase::new("example", vec!["ex", "example"])}
    }

    async fn generate_impl(
        &self,
        schema: ArcSchema,
        options: GeneratorOptions,
    ) -> Result<GenerationResult> {
        // Schema is already Arc, no cloning needed
        let mut result = GenerationResult {
            files: Vec::new(),
            content: Some(format!("Generated from schema: {}", schema.name)),
            warnings: Vec::new(),
            stats: GenerationStats::default()};

        // Use options to control generation behavior
        if let Some(output_dir) = &options.output_dir {
            result.files.push(format!("{output_dir}/schema.generated"));
        }

        // Process schema based on options
        let classes_to_generate = schema.classes.len();

        result.stats.classes_generated = classes_to_generate;
        result.stats.slots_generated = schema.slots.len();

        // Add warnings based on options
        if schema.classes.is_empty() {
            result.warnings.push("No classes found in schema for strict mode generation".to_string());
        }

        Ok(result)
    }
}

impl_generator_v2!(ExampleGeneratorV2, "example", vec!["ex", "example"]);

/// Parallel generator executor
pub struct ParallelGeneratorExecutor {
    registry: Arc<GeneratorRegistry>}

impl ParallelGeneratorExecutor {
    /// Create new executor
    pub fn new(registry: Arc<GeneratorRegistry>) -> Self {
        Self { registry }
    }

    /// Execute multiple generators in parallel
    pub async fn execute_all(
        &self,
        schema: ArcSchema,
        generator_names: Vec<String>,
        options: GeneratorOptions,
    ) -> Vec<(String, Result<GenerationResult>)> {
        use futures::future::join_all;

        let futures = generator_names.into_iter().map(|name| {
            let registry = Arc::clone(&self.registry);
            let schema = Arc::clone(&schema);
            let options = options.clone();

            async move {
                match registry.get(&name) {
                    Some(generator) => {
                        let result = generator.generate(schema, options).await;
                        (name, result)
                    }
                    None => (
                        name.clone(),
                        Err(linkml_core::error::LinkMLError::other(format!(
                            "Generator '{}' not found",
                            name
                        ))),
                    )}
            }
        });

        join_all(futures).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::prelude::*;

    #[tokio::test]
    async fn test_example_generator() {
        let schema = Arc::new(SchemaDefinition {
            id: "test".to_string(),
            name: "TestSchema".to_string(),
            ..Default::default()
        });

        let generator = ExampleGeneratorV2::new();
        let result = generator
            .generate(schema, GeneratorOptions::default())
            .await
            .expect("should generate successfully: {}");

        assert!(
            result
                .content
                .expect("should have content: {}")
                .contains("TestSchema")
        );
    }

    #[test]
    fn test_generator_registry() {
        let registry = GeneratorRegistry::new();
        registry.register(ExampleGeneratorV2::new());

        assert!(registry.get("example").is_some());
        assert_eq!(registry.list(), vec!["example"]);
    }

    #[tokio::test]
    async fn test_parallel_execution() {
        let registry = Arc::new(GeneratorRegistry::new());
        registry.register(ExampleGeneratorV2::new());

        let executor = ParallelGeneratorExecutor::new(registry);
        let schema = Arc::new(SchemaDefinition::default());

        let results = executor
            .execute_all(
                schema,
                vec!["example".to_string()],
                GeneratorOptions::default(),
            )
            .await;

        assert_eq!(results.len(), 1);
        assert!(results[0].1.is_ok());
    }
}