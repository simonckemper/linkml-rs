//! Enhanced import resolution for LinkML schemas
//!
//! This module provides advanced import resolution capabilities including:
//! - URL-based imports
//! - Import aliases and mappings
//! - Selective imports
//! - Conflict resolution
//! - Version checking

use linkml_core::{
    error::{LinkMLError, Result},
    settings::{ImportResolutionStrategy, ImportSettings},
    types::{SchemaDefinition, ClassDefinition, SlotDefinition},
};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use std::pin::Pin;
use std::future::Future;

/// Import specification with advanced options
#[derive(Debug, Clone)]
pub struct ImportSpec {
    /// The import path/URL
    pub path: String,
    /// Alias for the imported schema
    pub alias: Option<String>,
    /// Specific elements to import
    pub only: Option<Vec<String>>,
    /// Elements to exclude
    pub exclude: Option<Vec<String>>,
    /// Prefix to apply to imported elements
    pub prefix: Option<String>,
}

impl From<String> for ImportSpec {
    fn from(path: String) -> Self {
        Self {
            path,
            alias: None,
            only: None,
            exclude: None,
            prefix: None,
        }
    }
}

/// Enhanced import resolver with advanced capabilities
pub struct ImportResolverV2 {
    /// Cache of resolved schemas
    cache: Arc<RwLock<HashMap<String, SchemaDefinition>>>,
    /// Import settings from schema
    settings: Arc<RwLock<ImportSettings>>,
    /// HTTP client for URL imports
    http_client: reqwest::Client,
    /// Visited imports for circular dependency detection
    visited_stack: Arc<RwLock<Vec<String>>>,
}

impl ImportResolverV2 {
    /// Create a new import resolver with default settings
    #[must_use]
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            settings: Arc::new(RwLock::new(ImportSettings::default())),
            http_client: reqwest::Client::new(),
            visited_stack: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Create with schema settings
    #[must_use]
    pub fn with_settings(settings: ImportSettings) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            settings: Arc::new(RwLock::new(settings)),
            http_client: reqwest::Client::new(),
            visited_stack: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Update import settings
    pub fn set_settings(&self, settings: ImportSettings) {
        *self.settings.write() = settings;
    }
    
    /// Resolve all imports in a schema
    pub async fn resolve_imports(&self, schema: &SchemaDefinition) -> Result<SchemaDefinition> {
        let mut resolved = schema.clone();
        
        // Apply settings from schema if available, merging with existing settings
        if let Some(schema_settings) = &schema.settings {
            if let Some(import_settings) = &schema_settings.imports {
                let mut merged_settings = self.settings.read().clone();
                
                // Merge aliases
                for (alias, path) in &import_settings.aliases {
                    merged_settings.aliases.insert(alias.clone(), path.clone());
                }
                
                // Use schema settings but preserve existing search paths if schema doesn't specify
                if !import_settings.search_paths.is_empty() {
                    merged_settings.search_paths.extend(import_settings.search_paths.clone());
                }
                
                // Override other settings
                if import_settings.follow_imports.is_some() {
                    merged_settings.follow_imports = import_settings.follow_imports;
                }
                if import_settings.max_import_depth.is_some() {
                    merged_settings.max_import_depth = import_settings.max_import_depth;
                }
                if import_settings.cache_imports.is_some() {
                    merged_settings.cache_imports = import_settings.cache_imports;
                }
                if import_settings.resolution_strategy.is_some() {
                    merged_settings.resolution_strategy = import_settings.resolution_strategy;
                }
                if import_settings.base_url.is_some() {
                    merged_settings.base_url = import_settings.base_url.clone();
                }
                
                self.set_settings(merged_settings);
            }
        }
        
        // Check if imports should be followed
        let settings = self.settings.read();
        if !settings.should_follow_imports() {
            return Ok(resolved);
        }
        
        let max_depth = settings.max_import_depth.unwrap_or(10);
        drop(settings);
        
        // Resolve imports recursively
        self.resolve_imports_recursive(&mut resolved, 0, max_depth).await?;
        
        Ok(resolved)
    }
    
    /// Resolve imports recursively
    fn resolve_imports_recursive<'a>(
        &'a self,
        schema: &'a mut SchemaDefinition,
        depth: usize,
        max_depth: usize,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
        if depth >= max_depth {
            return Err(LinkMLError::import(
                &schema.name,
                format!("Maximum import depth ({}) exceeded", max_depth),
            ));
        }
        
        // Convert imports to ImportSpecs
        let import_specs: Vec<ImportSpec> = schema.imports
            .iter()
            .map(|imp| self.parse_import_spec(imp))
            .collect();
        
        // Process each import
        for spec in import_specs {
            // Check for circular dependencies
            {
                let stack = self.visited_stack.read();
                if stack.contains(&spec.path) {
                    return Err(LinkMLError::import(
                        &spec.path,
                        format!("Circular import detected: {} -> {}", 
                            stack.join(" -> "), spec.path),
                    ));
                }
            }
            
            // Add to visited stack
            self.visited_stack.write().push(spec.path.clone());
            
            // Load the imported schema
            let mut imported = self.load_import(&spec).await?;
            
            // Recursively resolve imports in the imported schema
            self.resolve_imports_recursive(&mut imported, depth + 1, max_depth).await?;
            
            // Merge into current schema
            self.merge_schema(schema, imported, &spec)?;
            
            // Remove from visited stack
            self.visited_stack.write().pop();
        }
        
        Ok(())
        })
    }
    
    /// Parse an import specification
    fn parse_import_spec(&self, import: &str) -> ImportSpec {
        // For now, simple string to ImportSpec conversion
        // TODO: Support advanced syntax like "base as b" or "base[Class1,Class2]"
        ImportSpec::from(import.to_string())
    }
    
    /// Load an imported schema
    async fn load_import(&self, spec: &ImportSpec) -> Result<SchemaDefinition> {
        // Check aliases
        let import_path = {
            let settings = self.settings.read();
            settings.aliases.get(&spec.path)
                .cloned()
                .unwrap_or_else(|| spec.path.clone())
        };
        
        // Check cache
        {
            let cache = self.cache.read();
            if let Some(schema) = cache.get(&import_path) {
                return Ok(schema.clone());
            }
        }
        
        // Load schema based on type (URL or file)
        let schema = if import_path.starts_with("http://") || import_path.starts_with("https://") {
            self.load_url_import(&import_path).await?
        } else {
            self.load_file_import(&import_path).await?
        };
        
        // Cache if enabled
        let settings = self.settings.read();
        if settings.cache_imports.unwrap_or(true) {
            drop(settings);
            let mut cache = self.cache.write();
            cache.insert(import_path, schema.clone());
        }
        
        Ok(schema)
    }
    
    /// Load schema from URL
    async fn load_url_import(&self, url_str: &str) -> Result<SchemaDefinition> {
        // Resolve relative URLs against base URL if available
        let final_url = {
            let settings = self.settings.read();
            if let Some(base_url) = &settings.base_url {
                // Try to resolve relative to base
                if let Ok(base) = url::Url::parse(base_url) {
                    if let Ok(resolved) = base.join(url_str) {
                        resolved.to_string()
                    } else {
                        url_str.to_string()
                    }
                } else {
                    url_str.to_string()
                }
            } else {
                url_str.to_string()
            }
        };
        
        let response = self.http_client.get(&final_url)
            .send()
            .await
            .map_err(|e| LinkMLError::import(&final_url, format!("Failed to fetch URL: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(LinkMLError::import(
                &final_url,
                format!("HTTP error: {}", response.status()),
            ));
        }
        
        let content = response.text()
            .await
            .map_err(|e| LinkMLError::import(&final_url, format!("Failed to read response: {}", e)))?;
        
        // Parse based on URL extension
        self.parse_schema_content(&content, &final_url)
    }
    
    /// Load schema from file
    async fn load_file_import(&self, path: &str) -> Result<SchemaDefinition> {
        let file_path = self.resolve_file_path(path)?;
        
        let content = fs::read_to_string(&file_path)
            .await
            .map_err(|e| LinkMLError::import(path, format!("Failed to read file: {}", e)))?;
        
        self.parse_schema_content(&content, path)
    }
    
    /// Resolve file path using search paths and resolution strategy
    fn resolve_file_path(&self, import: &str) -> Result<PathBuf> {
        let settings = self.settings.read();
        let strategy = settings.get_resolution_strategy();
        let search_paths = &settings.search_paths;
        
        // Common file extensions to try
        let extensions = ["yaml", "yml", "json"];
        
        match strategy {
            ImportResolutionStrategy::Relative => {
                // Try relative to current file first
                // If we have search paths, use the first one as the base
                // Otherwise use current directory
                let base_paths = if search_paths.is_empty() {
                    vec![PathBuf::from(".")]
                } else {
                    vec![PathBuf::from(&search_paths[0])]
                };
                self.find_in_paths(import, &base_paths, &extensions)
            }
            ImportResolutionStrategy::Absolute => {
                // Only use search paths
                let paths: Vec<PathBuf> = search_paths.iter()
                    .map(|s| PathBuf::from(s))
                    .collect();
                self.find_in_paths(import, &paths, &extensions)
            }
            ImportResolutionStrategy::Mixed => {
                // Try relative first, then search paths
                self.find_in_paths(import, &[PathBuf::from(".")], &extensions)
                    .or_else(|_| {
                        let paths: Vec<PathBuf> = search_paths.iter()
                            .map(|s| PathBuf::from(s))
                            .collect();
                        self.find_in_paths(import, &paths, &extensions)
                    })
            }
        }
    }
    
    /// Find file in given paths
    fn find_in_paths(&self, import: &str, paths: &[PathBuf], extensions: &[&str]) -> Result<PathBuf> {
        for base_path in paths {
            // Try with original name
            let path = base_path.join(import);
            if path.exists() {
                return Ok(path);
            }
            
            // Try with extensions
            for ext in extensions {
                let path = base_path.join(format!("{}.{}", import, ext));
                if path.exists() {
                    return Ok(path);
                }
            }
        }
        
        Err(LinkMLError::import(
            import,
            format!("File not found in paths: {:?}", paths),
        ))
    }
    
    /// Parse schema content based on format
    fn parse_schema_content(&self, content: &str, source: &str) -> Result<SchemaDefinition> {
        use crate::parser::{SchemaParser, YamlParser, JsonParser};
        
        // Determine format from extension
        if source.ends_with(".json") {
            let parser = JsonParser::new();
            parser.parse_str(content)
        } else {
            // Default to YAML
            let parser = YamlParser::new();
            parser.parse_str(content)
        }
    }
    
    /// Merge imported schema into target schema
    fn merge_schema(
        &self,
        target: &mut SchemaDefinition,
        mut source: SchemaDefinition,
        spec: &ImportSpec,
    ) -> Result<()> {
        // Apply prefix if specified
        if let Some(prefix) = &spec.prefix {
            self.apply_prefix(&mut source, prefix);
        }
        
        // Filter elements based on only/exclude
        if spec.only.is_some() || spec.exclude.is_some() {
            self.filter_schema(&mut source, spec);
        }
        
        // Merge prefixes
        for (name, def) in source.prefixes {
            match target.prefixes.get(&name) {
                Some(existing) if existing != &def => {
                    // Conflict - use fully qualified name
                    let qualified_name = format!("{}_{}", spec.alias.as_ref().unwrap_or(&source.name), name);
                    target.prefixes.insert(qualified_name, def);
                }
                None => {
                    target.prefixes.insert(name, def);
                }
                _ => {} // Same definition, skip
            }
        }
        
        // Merge classes with conflict detection
        for (name, class) in source.classes {
            let qualified_name = self.get_qualified_name(&name, &spec, &source.name);
            if target.classes.contains_key(&name) {
                // Conflict - use qualified name
                target.classes.insert(qualified_name, class);
            } else {
                target.classes.insert(name, class);
            }
        }
        
        // Merge slots
        for (name, slot) in source.slots {
            let qualified_name = self.get_qualified_name(&name, &spec, &source.name);
            if target.slots.contains_key(&name) {
                target.slots.insert(qualified_name, slot);
            } else {
                target.slots.insert(name, slot);
            }
        }
        
        // Merge types
        for (name, type_def) in source.types {
            let qualified_name = self.get_qualified_name(&name, &spec, &source.name);
            if target.types.contains_key(&name) {
                target.types.insert(qualified_name, type_def);
            } else {
                target.types.insert(name, type_def);
            }
        }
        
        // Merge enums
        for (name, enum_def) in source.enums {
            let qualified_name = self.get_qualified_name(&name, &spec, &source.name);
            if target.enums.contains_key(&name) {
                target.enums.insert(qualified_name, enum_def);
            } else {
                target.enums.insert(name, enum_def);
            }
        }
        
        Ok(())
    }
    
    /// Apply prefix to all elements in schema
    fn apply_prefix(&self, schema: &mut SchemaDefinition, prefix: &str) {
        // Prefix all class names
        let classes: Vec<(String, ClassDefinition)> = schema.classes
            .drain(..)
            .map(|(name, class)| (format!("{}_{}", prefix, name), class))
            .collect();
        schema.classes.extend(classes);
        
        // Prefix all slot names
        let slots: Vec<(String, SlotDefinition)> = schema.slots
            .drain(..)
            .map(|(name, slot)| (format!("{}_{}", prefix, name), slot))
            .collect();
        schema.slots.extend(slots);
        
        // Update references in classes
        for class in schema.classes.values_mut() {
            if let Some(is_a) = &mut class.is_a {
                *is_a = format!("{}_{}", prefix, is_a);
            }
            class.mixins = class.mixins.iter()
                .map(|m| format!("{}_{}", prefix, m))
                .collect();
            class.slots = class.slots.iter()
                .map(|s| format!("{}_{}", prefix, s))
                .collect();
        }
    }
    
    /// Filter schema elements based on only/exclude lists
    fn filter_schema(&self, schema: &mut SchemaDefinition, spec: &ImportSpec) {
        if let Some(only) = &spec.only {
            // Keep only specified elements
            schema.classes.retain(|name, _| only.contains(name));
            schema.slots.retain(|name, _| only.contains(name));
            schema.types.retain(|name, _| only.contains(name));
            schema.enums.retain(|name, _| only.contains(name));
        }
        
        if let Some(exclude) = &spec.exclude {
            // Remove excluded elements
            for name in exclude {
                schema.classes.shift_remove(name);
                schema.slots.shift_remove(name);
                schema.types.shift_remove(name);
                schema.enums.shift_remove(name);
            }
        }
    }
    
    /// Get qualified name for an element
    fn get_qualified_name(&self, name: &str, spec: &ImportSpec, schema_name: &str) -> String {
        if let Some(alias) = &spec.alias {
            format!("{}_{}", alias, name)
        } else {
            format!("{}_{}", schema_name, name)
        }
    }
    
    /// Clear the import cache
    pub fn clear_cache(&self) {
        self.cache.write().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_enhanced_import_resolver() {
        // Create test schemas
        let temp_dir = TempDir::new().expect("should create temporary directory");
        let base_path = temp_dir.path();
        
        // Base schema
        let base_schema = r#"
id: https://example.org/base
name: base
version: 1.0.0
classes:
  BaseClass:
    name: BaseClass
    description: Base class
  SharedClass:
    name: SharedClass
    description: This class name conflicts
slots:
  base_slot:
    name: base_slot
    range: string
"#;
        
        tokio::fs::write(base_path.join("base.yaml"), base_schema).await.expect("should write base schema");
        
        // Another schema with conflicts
        let other_schema = r#"
id: https://example.org/other
name: other
classes:
  OtherClass:
    name: OtherClass
    description: Other class
  SharedClass:
    name: SharedClass
    description: Different shared class
"#;
        
        tokio::fs::write(base_path.join("other.yaml"), other_schema).await.expect("should write other schema");
        
        // Main schema with imports
        let main_schema = r#"
id: https://example.org/main
name: main
settings:
  imports:
    search_paths:
      - "."
    cache_imports: true
    resolution_strategy: mixed
imports:
  - base
  - other
classes:
  MainClass:
    name: MainClass
    is_a: BaseClass
"#;
        
        // Parse and resolve
        use crate::parser::{SchemaParser, YamlParser};
        let parser = YamlParser::new();
        let mut schema = parser.parse_str(main_schema).expect("should parse main schema");
        
        // Set base path for resolver
        if let Some(settings) = &mut schema.settings {
            if let Some(imports) = &mut settings.imports {
                imports.search_paths = vec![base_path.to_str().expect("temp dir path should be valid UTF-8").to_string()];
            }
        }
        
        let resolver = ImportResolverV2::new();
        let resolved = resolver.resolve_imports(&schema).await.expect("should resolve imports");
        
        // Check that all elements were imported
        assert!(resolved.classes.contains_key("BaseClass"));
        assert!(resolved.classes.contains_key("OtherClass"));
        assert!(resolved.classes.contains_key("MainClass"));
        assert!(resolved.slots.contains_key("base_slot"));
        
        // Check that conflicting class was handled
        assert!(resolved.classes.contains_key("SharedClass"));
        // One of the conflicts should have been renamed
        assert!(resolved.classes.contains_key("other_SharedClass") || 
                resolved.classes.len() == 5); // All classes including renamed
    }
    
    #[tokio::test]
    async fn test_circular_import_detection() {
        let temp_dir = TempDir::new().expect("should create temporary directory");
        let base_path = temp_dir.path();
        
        // Schema A imports B
        let schema_a = r#"
id: https://example.org/a
name: a
imports:
  - b
"#;
        
        // Schema B imports A (circular)
        let schema_b = r#"
id: https://example.org/b
name: b
imports:
  - a
"#;
        
        tokio::fs::write(base_path.join("a.yaml"), schema_a).await.expect("should write schema a");
        tokio::fs::write(base_path.join("b.yaml"), schema_b).await.expect("should write schema b");
        
        // Try to resolve - should fail
        use crate::parser::{SchemaParser, YamlParser};
        let parser = YamlParser::new();
        let schema = parser.parse_str(schema_a).expect("should parse schema a");
        
        let mut settings = ImportSettings::default();
        settings.search_paths = vec![base_path.to_str().unwrap().to_string()];
        
        let resolver = ImportResolverV2::with_settings(settings);
        let result = resolver.resolve_imports(&schema).await;
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Circular import"));
    }
}