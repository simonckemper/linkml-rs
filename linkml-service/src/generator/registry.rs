//! Generator registry for managing available generators

use super::traits::{Generator, GeneratorError, GeneratorResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Registry for managing code generators
pub struct GeneratorRegistry {
    /// Registered generators by name
    generators: RwLock<HashMap<String, Arc<dyn Generator>>>,
}

impl GeneratorRegistry {
    /// Create a new empty registry
    #[must_use]
    pub fn new() -> Self {
        Self {
            generators: RwLock::new(HashMap::new()),
        }
    }

    /// Create a registry with default generators
    pub async fn with_defaults() -> Self {
        let registry = Self::new();

        use super::{
            GraphQLGenerator, HtmlGenerator, JavaScriptGenerator, JsonSchemaGenerator, 
            OpenApiGenerator, PydanticGenerator, PythonDataclassGenerator, RustGenerator, 
            SQLGenerator, TypeQLGenerator, TypeScriptGenerator,
        };

        // Register all available generators
        let generators: Vec<Arc<dyn Generator>> = vec![
            Arc::new(PythonDataclassGenerator::new()),
            Arc::new(PydanticGenerator::new()),
            Arc::new(TypeScriptGenerator::new()),
            Arc::new(JavaScriptGenerator::new()),
            Arc::new(GraphQLGenerator::new()),
            Arc::new(RustGenerator::new()),
            Arc::new(TypeQLGenerator::new()),
            Arc::new(HtmlGenerator::new()),
            Arc::new(JsonSchemaGenerator::new()),
            Arc::new(OpenApiGenerator::new()),
            Arc::new(SQLGenerator::new()),
        ];

        for generator in generators {
            if let Err(e) = registry.register(generator).await {
                eprintln!("Failed to register generator: {}", e);
            }
        }

        registry
    }

    /// Register a generator
    pub async fn register(&self, generator: Arc<dyn Generator>) -> GeneratorResult<()> {
        let name = generator.name().to_string();

        let mut generators = self.generators.write().await;

        if generators.contains_key(&name) {
            return Err(GeneratorError::Configuration(format!(
                "Generator '{name}' is already registered"
            )));
        }

        generators.insert(name, generator);
        Ok(())
    }

    /// Unregister a generator
    pub async fn unregister(&self, name: &str) -> GeneratorResult<()> {
        let mut generators = self.generators.write().await;

        if generators.remove(name).is_none() {
            return Err(GeneratorError::Configuration(format!(
                "Generator '{name}' not found"
            )));
        }

        Ok(())
    }

    /// Get a generator by name
    pub async fn get(&self, name: &str) -> Option<Arc<dyn Generator>> {
        let generators = self.generators.read().await;
        generators.get(name).cloned()
    }

    /// Get all registered generator names
    pub async fn list_generators(&self) -> Vec<String> {
        let generators = self.generators.read().await;
        generators.keys().cloned().collect()
    }

    /// Get generator information
    pub async fn get_info(&self, name: &str) -> Option<GeneratorInfo> {
        let generators = self.generators.read().await;

        generators.get(name).map(|generator| GeneratorInfo {
            name: generator.name().to_string(),
            description: generator.description().to_string(),
            file_extensions: generator
                .file_extensions()
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
        })
    }

    /// Get information for all generators
    pub async fn list_info(&self) -> Vec<GeneratorInfo> {
        let generators = self.generators.read().await;

        generators
            .values()
            .map(|generator| GeneratorInfo {
                name: generator.name().to_string(),
                description: generator.description().to_string(),
                file_extensions: generator
                    .file_extensions()
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect(),
            })
            .collect()
    }
}

impl Default for GeneratorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about a registered generator
#[derive(Debug, Clone)]
pub struct GeneratorInfo {
    /// Generator name
    pub name: String,

    /// Generator description
    pub description: String,

    /// File extensions produced
    pub file_extensions: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::traits::{GeneratedOutput, GeneratorOptions};
    use async_trait::async_trait;
    use linkml_core::prelude::SchemaDefinition;

    struct TestGenerator {
        name: String,
    }

    #[async_trait]
    impl Generator for TestGenerator {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &'static str {
            "Test generator"
        }

        fn file_extensions(&self) -> Vec<&str> {
            vec![".test"]
        }

        async fn generate(
            &self,
            _schema: &SchemaDefinition,
            _options: &GeneratorOptions,
        ) -> GeneratorResult<Vec<GeneratedOutput>> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn test_registry_operations() {
        let registry = GeneratorRegistry::new();

        // Register a generator
        let generator = Arc::new(TestGenerator {
            name: "test".to_string(),
        });

        registry.register(generator.clone()).await.unwrap();

        // Should be able to retrieve it
        let retrieved = registry.get("test").await.unwrap();
        assert_eq!(retrieved.name(), "test");

        // List should include it
        let names = registry.list_generators().await;
        assert!(names.contains(&"test".to_string()));

        // Get info
        let info = registry.get_info("test").await.unwrap();
        assert_eq!(info.name, "test");
        assert_eq!(info.description, "Test generator");

        // Unregister
        registry.unregister("test").await.unwrap();
        assert!(registry.get("test").await.is_none());
    }

    #[tokio::test]
    async fn test_duplicate_registration() {
        let registry = GeneratorRegistry::new();

        let gen1 = Arc::new(TestGenerator {
            name: "test".to_string(),
        });
        let gen2 = Arc::new(TestGenerator {
            name: "test".to_string(),
        });

        registry.register(gen1).await.unwrap();

        // Second registration should fail
        let result = registry.register(gen2).await;
        assert!(result.is_err());
    }
}
