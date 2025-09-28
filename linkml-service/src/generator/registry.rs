//! Generator registry for managing available generators

use super::traits::{Generator, GeneratorError, GeneratorResult};
use crate::plugin::{GeneratorPlugin, PluginManager, PluginStatus, PluginType};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// Registry for managing code generators
pub struct GeneratorRegistry {
    /// Registered generators by name
    generators: RwLock<HashMap<String, Arc<dyn Generator>>>,
    /// Plugin manager for loading generator plugins
    plugin_manager: Option<Arc<Mutex<PluginManager>>>,
    /// Plugin-based generators
    plugin_generators: RwLock<HashMap<String, Arc<dyn GeneratorPlugin>>>,
}

impl GeneratorRegistry {
    /// Create a new empty registry
    #[must_use]
    pub fn new() -> Self {
        Self {
            generators: RwLock::new(HashMap::new()),
            plugin_manager: None,
            plugin_generators: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new registry with plugin support
    pub fn with_plugin_manager(plugin_manager: Arc<Mutex<PluginManager>>) -> Self {
        Self {
            generators: RwLock::new(HashMap::new()),
            plugin_manager: Some(plugin_manager),
            plugin_generators: RwLock::new(HashMap::new()),
        }
    }

    /// Create a registry with default generators
    pub async fn with_defaults() -> Self {
        use super::{
            CsvGenerator, ExcelGenerator, GoGenerator, GraphQLGenerator, GraphvizGenerator,
            HtmlGenerator, JavaGenerator, JavaScriptGenerator, JsonLdContextGenerator,
            JsonLdContextGeneratorConfig, JsonLdGenerator, JsonSchemaGenerator, MarkdownGenerator,
            MermaidDiagramType, MermaidGenerator, NamespaceManagerGenerator,
            NamespaceManagerGeneratorConfig, OpenApiGenerator, PlantUmlGenerator, PrefixMapFormat,
            PrefixMapGenerator, PrefixMapGeneratorConfig, ProtobufGenerator, PydanticGenerator,
            PythonDataclassGenerator, RdfGenerator, RustGenerator, SQLAlchemyGenerator,
            SQLAlchemyGeneratorConfig, SQLGenerator, ShExGenerator, ShaclGenerator,
            SparqlGenerator, SssomFormat, SssomGenerator, SssomGeneratorConfig, SummaryFormat,
            SummaryGenerator, SummaryGeneratorConfig, TargetLanguage as NsTargetLanguage,
            TypeScriptGenerator, ValidationFramework, YamlValidatorGenerator,
            YamlValidatorGeneratorConfig, YumlGenerator, typeql_generator::create_typeql_generator,
        };

        let registry = Self::new();

        // Register all available generators
        let generators: Vec<Arc<dyn Generator>> = vec![
            Arc::new(PythonDataclassGenerator::new()),
            Arc::new(PydanticGenerator::new()),
            Arc::new(TypeScriptGenerator::new()),
            Arc::new(JavaScriptGenerator::new()),
            Arc::new(JavaGenerator::new()),
            Arc::new(CsvGenerator::new()),
            Arc::new(CsvGenerator::tsv()),
            Arc::new(GoGenerator::new()),
            Arc::new(ExcelGenerator::new()),
            Arc::new(GraphQLGenerator::new()),
            Arc::new(GraphvizGenerator::new()),
            Arc::new(RustGenerator::new()),
            Arc::new(create_typeql_generator()),
            Arc::new(HtmlGenerator::new()),
            Arc::new(JsonSchemaGenerator::new()),
            Arc::new(JsonLdGenerator::new()),
            Arc::new(JsonLdContextGenerator::new(
                JsonLdContextGeneratorConfig::default(),
            )),
            Arc::new(MarkdownGenerator::new()),
            Arc::new(MermaidGenerator::new()), // ER diagram (default)
            Arc::new(MermaidGenerator::new().with_diagram_type(MermaidDiagramType::ClassDiagram)),
            Arc::new(OpenApiGenerator::new()),
            Arc::new(RdfGenerator::new()),    // OWL mode
            Arc::new(RdfGenerator::rdfs()),   // RDFS mode
            Arc::new(RdfGenerator::simple()), // Simple RDF mode
            Arc::new(ProtobufGenerator::new()),
            Arc::new(ShaclGenerator::new()),
            Arc::new(ShExGenerator::new()),
            Arc::new(SparqlGenerator::new()),
            Arc::new(SQLAlchemyGenerator::new(
                SQLAlchemyGeneratorConfig::default(),
            )),
            Arc::new(SQLGenerator::new()),
            Arc::new(PlantUmlGenerator::new()),
            Arc::new(YumlGenerator::new()),
            Arc::new(PrefixMapGenerator::new(PrefixMapGeneratorConfig::default())), // Simple JSON format
            Arc::new(PrefixMapGenerator::new(PrefixMapGeneratorConfig {
                format: PrefixMapFormat::Extended,
                include_metadata: true,
                ..PrefixMapGeneratorConfig::default()
            })), // Extended format
            Arc::new(PrefixMapGenerator::new(PrefixMapGeneratorConfig {
                format: PrefixMapFormat::Turtle,
                ..PrefixMapGeneratorConfig::default()
            })), // Turtle format
            Arc::new(YamlValidatorGenerator::new(
                YamlValidatorGeneratorConfig::default(),
            )), // JSON Schema format
            Arc::new(YamlValidatorGenerator::new(YamlValidatorGeneratorConfig {
                framework: ValidationFramework::Cerberus,
                ..YamlValidatorGeneratorConfig::default()
            })), // Cerberus format
            Arc::new(YamlValidatorGenerator::new(YamlValidatorGeneratorConfig {
                framework: ValidationFramework::Joi,
                ..YamlValidatorGeneratorConfig::default()
            })), // Joi format
            Arc::new(NamespaceManagerGenerator::new(
                NamespaceManagerGeneratorConfig::default(),
            )), // Python namespace manager
            Arc::new(NamespaceManagerGenerator::new(
                NamespaceManagerGeneratorConfig {
                    target_language: NsTargetLanguage::JavaScript,
                    ..NamespaceManagerGeneratorConfig::default()
                },
            )), // JavaScript namespace manager
            Arc::new(NamespaceManagerGenerator::new(
                NamespaceManagerGeneratorConfig {
                    target_language: NsTargetLanguage::Rust,
                    thread_safe: true,
                    ..NamespaceManagerGeneratorConfig::default()
                },
            )), // Rust namespace manager
            Arc::new(SssomGenerator::new(SssomGeneratorConfig::default())), // SSSOM TSV format
            Arc::new(SssomGenerator::new(SssomGeneratorConfig {
                format: SssomFormat::Json,
                ..SssomGeneratorConfig::default()
            })), // SSSOM JSON format
            Arc::new(SummaryGenerator::new(SummaryGeneratorConfig::default())), // Summary TSV format
            Arc::new(SummaryGenerator::new(SummaryGeneratorConfig {
                format: SummaryFormat::Markdown,
                detailed: true,
                ..SummaryGeneratorConfig::default()
            })), // Summary Markdown format
            Arc::new(SummaryGenerator::new(SummaryGeneratorConfig {
                format: SummaryFormat::Json,
                detailed: true,
                complexity_metrics: true,
                ..SummaryGeneratorConfig::default()
            })), // Summary JSON format
                                                                                // ProjectGenerator is not implemented yet
                                                                                // Arc::new(ProjectGenerator::new(Default::default())), // Project generator (Python)
                                                                                // Arc::new(ProjectGenerator::new(ProjectGeneratorConfig {
                                                                                //     target: ProjectTarget::TypeScript,
                                                                                //     ..Default::default()
                                                                                // })), // Project generator (TypeScript)
                                                                                // Arc::new(ProjectGenerator::new(ProjectGeneratorConfig {
                                                                                //     target: ProjectTarget::Rust,
                                                                                //     ..Default::default()
                                                                                // })), // Project generator (Rust)
        ];

        for generator in generators {
            if let Err(e) = registry.register(generator).await {
                eprintln!("Failed to register generator: {e}");
            }
        }

        registry
    }

    /// Register a generator
    ///
    /// # Errors
    ///
    /// Returns `GeneratorError::Configuration` if a generator with the same name is already registered.
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
    ///
    /// # Errors
    ///
    /// Returns `GeneratorError::Configuration` if no generator with the specified name is found.
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

    /// Get a generator by name (alias for get method)
    ///
    /// This method provides compatibility with code expecting a `get_generator` method.
    pub async fn get_generator(&self, name: &str) -> Option<Arc<dyn Generator>> {
        self.get(name).await
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

    /// Load generator plugins from the plugin manager
    ///
    /// # Errors
    ///
    /// Returns error if plugin loading fails due to invalid plugin files, missing dependencies, or
    /// incompatible plugin versions.
    pub async fn load_generator_plugins(&self) -> GeneratorResult<usize> {
        if let Some(plugin_manager) = &self.plugin_manager {
            let manager = plugin_manager.lock().await;
            let generator_plugins = manager.get_plugins_by_type(PluginType::Generator);

            let mut count = 0;
            let mut failed_plugins: Vec<String> = Vec::new();

            for plugin in generator_plugins {
                let plugin_locked = match plugin.lock() {
                    Ok(guard) => guard,
                    Err(_) => {
                        failed_plugins.push("Failed to lock plugin mutex".to_string());
                        continue;
                    }
                };

                // Check plugin status and capabilities
                let status = plugin_locked.status();
                if status != PluginStatus::Ready && status != PluginStatus::Initializing {
                    failed_plugins.push(format!("Plugin is not active: {:?}", status));
                    continue;
                }

                // Get plugin info for validation
                let plugin_info = plugin_locked.info();

                // Validate plugin has generator capability
                if !plugin_info
                    .capabilities
                    .contains(&crate::plugin::PluginCapability::CodeGeneration)
                {
                    failed_plugins.push(format!(
                        "{}: Plugin lacks generation capability",
                        plugin_info.name
                    ));
                    continue;
                }

                count += 1;
                tracing::debug!("Loaded generator plugin: {}", plugin_info.name);
            }

            // Report any failures but don't fail completely if some plugins loaded
            if !failed_plugins.is_empty() {
                tracing::warn!(
                    "Some generator plugins failed to load: {}",
                    failed_plugins.join(", ")
                );
            }

            Ok(count)
        } else {
            Ok(0)
        }
    }

    /// Register a plugin-based generator
    ///
    /// # Errors
    ///
    /// Returns `GeneratorError::Configuration` if a plugin generator with the same name is already registered.
    pub async fn register_plugin_generator(
        &self,
        name: String,
        generator: Arc<dyn GeneratorPlugin>,
    ) -> GeneratorResult<()> {
        let mut plugin_generators = self.plugin_generators.write().await;

        if plugin_generators.contains_key(&name) {
            return Err(GeneratorError::Configuration(format!(
                "Plugin generator '{name}' is already registered"
            )));
        }

        plugin_generators.insert(name, generator);
        Ok(())
    }

    /// Get a plugin-based generator by name
    pub async fn get_plugin_generator(&self, name: &str) -> Option<Arc<dyn GeneratorPlugin>> {
        let plugin_generators = self.plugin_generators.read().await;
        plugin_generators.get(name).cloned()
    }

    /// List all plugin-based generators
    pub async fn list_plugin_generators(&self) -> Vec<String> {
        let plugin_generators = self.plugin_generators.read().await;
        plugin_generators.keys().cloned().collect()
    }

    /// Get combined list of all generators (built-in and plugin-based)
    pub async fn list_all_generators(&self) -> Vec<String> {
        let mut all_generators = self.list_generators().await;
        let plugin_generators = self.list_plugin_generators().await;
        all_generators.extend(plugin_generators);
        all_generators.sort();
        all_generators.dedup();
        all_generators
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

    use linkml_core::{LinkMLError, types::SchemaDefinition};

    struct TestGenerator {
        name: String,
    }

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

        fn generate(&self, _schema: &SchemaDefinition) -> std::result::Result<String, LinkMLError> {
            Ok(String::new())
        }

        fn get_file_extension(&self) -> &str {
            ".test"
        }

        fn get_default_filename(&self) -> &str {
            "output.test"
        }

        fn validate_schema(&self, _schema: &SchemaDefinition) -> Result<(), LinkMLError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_registry_operations() -> anyhow::Result<()> {
        let registry = GeneratorRegistry::new();

        // Register a generator
        let generator = Arc::new(TestGenerator {
            name: "test".to_string(),
        });

        registry
            .register(generator.clone())
            .await
            .expect("should register generator: {}");

        // Should be able to retrieve it
        let retrieved = registry
            .get("test")
            .await
            .ok_or_else(|| anyhow::anyhow!("should retrieve generator"))?;
        assert_eq!(retrieved.name(), "test");

        // List should include it
        let names = registry.list_generators().await;
        assert!(names.contains(&"test".to_string()));

        // Get info
        let info = registry
            .get_info("test")
            .await
            .ok_or_else(|| anyhow::anyhow!("should get generator info"))?;
        assert_eq!(info.name, "test");
        assert_eq!(info.description, "Test generator");

        // Unregister
        registry
            .unregister("test")
            .await
            .expect("should unregister generator: {}");
        assert!(registry.get("test").await.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn test_duplicate_registration() -> anyhow::Result<()> {
        let registry = GeneratorRegistry::new();

        let gen1 = Arc::new(TestGenerator {
            name: "test".to_string(),
        });
        let gen2 = Arc::new(TestGenerator {
            name: "test".to_string(),
        });

        registry
            .register(gen1)
            .await
            .expect("should register first generator: {}");

        // Second registration should fail
        let result = registry.register(gen2).await;
        assert!(result.is_err());
        Ok(())
    }
}
