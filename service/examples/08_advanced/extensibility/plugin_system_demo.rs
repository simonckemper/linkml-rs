//! Example demonstrating the LinkML plugin system
//!
//! This example shows how to:
//! 1. Discover plugins from directories
//! 2. Load and initialize plugins
//! 3. Use plugin-based generators
//! 4. Manage plugin lifecycle

use linkml_core::prelude::*;
use linkml_service::parser::SchemaParser;
use linkml_service::plugin::{PluginContext, PluginManager, PluginType};
use logger_service::factory::create_development_logger;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use timestamp_core::factory::create_timestamp_service;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Initialize logger service
    let timestamp = create_timestamp_service();
    let logger = Arc::new(create_development_logger(timestamp).await?);

    // Create plugin manager
    let mut plugin_manager = PluginManager::new(logger.clone());

    println!(
        "=== LinkML Plugin System Demo ===
"
    );

    // 1. Discover plugins from various sources
    println!("1. Discovering plugins...");

    // Discover from current directory
    let current_dir = Path::new("./plugins");
    if current_dir.exists() {
        let discovered = plugin_manager.discover_plugins(current_dir).await?;
        println!("   Found {} plugins in ./plugins", discovered.len());
        for plugin_info in &discovered {
            println!(
                "   - {} v{} ({})",
                plugin_info.name,
                plugin_info.version,
                plugin_info.plugin_type.as_str()
            );
        }
    }

    // Discover from system directories
    let system_plugins = plugin_manager
        .discover_plugins(Path::new("/usr/local/share/linkml/plugins"))
        .await
        .unwrap_or_default();
    println!("   Found {} system plugins", system_plugins.len());

    // 2. Initialize all plugins
    println!(
        "
2. Initializing plugins..."
    );

    let plugin_context = PluginContext {
        config: HashMap::new(),
        working_dir: std::env::current_dir()?,
        temp_dir: std::env::temp_dir(),
    };

    plugin_manager
        .initialize_all(plugin_context.clone())
        .await?;
    println!("   All plugins initialized successfully");

    // 3. Demonstrate plugin usage with a custom generator
    println!(
        "
3. Using plugin-based generators..."
    );

    // Get all generator plugins
    let generator_plugins = plugin_manager.get_plugins_by_type(PluginType::Generator);
    println!(
        "   Available generator plugins: {}",
        generator_plugins.len()
    );

    // Example schema to generate code from
    let schema_yaml = r#"
id: https://example.com/plugin-demo
name: PluginDemo
description: Demo schema for plugin system

prefixes:
  linkml: https://w3id.org/linkml/
  demo: https://example.com/demo/

classes:
  PluginConfig:
    description: Configuration for a plugin
    attributes:
      id:
        identifier: true
        range: string
      name:
        range: string
        required: true
      version:
        range: string
        pattern: '^\d+\.\d+\.\d+$'
      enabled:
        range: boolean
      settings:
        range: string
        multivalued: true
"#;

    // Parse the schema
    let mut parser = SchemaParser::new();
    let schema = parser.parse(schema_yaml)?;

    // Use each generator plugin
    for plugin in generator_plugins {
        // Try to cast to GeneratorPlugin
        // In a real implementation, this would use the plugin trait properly
        println!(
            "
   Plugin: {}",
            plugin.info().name
        );
        println!("   Supported formats: (would list formats)");

        // Generate code (example)
        // let output = generator.generate(&schema, "default", HashMap::new()).await?;
        // println!("   Generated {} bytes of code", output.len());
    }

    // 4. Demonstrate plugin capabilities
    println!(
        "
4. Plugin capabilities..."
    );

    // Check for specific capabilities
    let all_plugins = plugin_manager.get_all();
    for plugin in &all_plugins {
        let info = plugin.info();
        println!(
            "
   Plugin: {}",
            info.name
        );
        println!("   Capabilities:");
        for cap in &info.capabilities {
            println!("     - {:?}", cap);
        }
    }

    // 5. Plugin health monitoring
    println!(
        "
5. Plugin health status..."
    );

    for plugin in &all_plugins {
        let status = plugin.status();
        println!("   {} - {:?}", plugin.info().name, status);
    }

    // 6. Demonstrate plugin configuration
    println!(
        "
6. Plugin configuration..."
    );

    // Example: Configure a specific plugin
    if let Some(example_plugin) = plugin_manager.get_plugin("example-generator") {
        let mut config = HashMap::new();
        config.insert("output_format".to_string(), serde_json::json!("typescript"));
        config.insert("include_comments".to_string(), serde_json::json!(true));
        config.insert("strict_mode".to_string(), serde_json::json!(false));

        match example_plugin.validate_config(&config) {
            Ok(_) => println!("   Configuration valid for example-generator"),
            Err(e) => println!("   Configuration error: {}", e),
        }
    }

    // 7. Plugin dependency checking
    println!(
        "
7. Checking plugin dependencies..."
    );

    // This would check if all plugin dependencies are satisfied
    // let dep_errors = plugin_manager.check_dependencies()?;
    // if dep_errors.is_empty() {
    //     println!("   All plugin dependencies satisfied");
    // } else {
    //     println!("   Dependency errors found:");
    //     for error in dep_errors {
    //         println!("     - {}: {}", error.plugin_id, error.reason);
    //     }
    // }

    // 8. Shutdown plugins
    println!(
        "
8. Shutting down plugins..."
    );

    plugin_manager.shutdown_all().await?;
    println!("   All plugins shut down successfully");

    // Example of creating a plugin manifest
    println!(
        "
=== Plugin Manifest Example ==="
    );
    let manifest_example = r#"
[plugin]
id = "custom-typescript-generator"
name = "Custom TypeScript Generator"
description = "Enhanced TypeScript code generator with additional features"
version = "1.2.0"
plugin_type = "Generator"
author = "Your Name"
license = "CC-BY-NC-4.0"
homepage = "https://github.com/yourusername/custom-ts-generator"
linkml_version = ">=2.0.0"

[[plugin.dependencies]]
id = "base-generator-utils"
version = ">=1.0.0"
optional = false

[[plugin.capabilities]]
"CodeGeneration"

[[plugin.capabilities]]
"AsyncOperations"

[[plugin.capabilities]]
"RuntimeConfiguration"

[entry_point]
type = "Native"
library = "libcustom_ts_generator.so"
symbol = "create_plugin"

[build]
command = "cargo build --release"
directory = "."

[requirements]
system = ["cargo", "rustc"]
"#;

    println!("{}", manifest_example);

    Ok(())
}
