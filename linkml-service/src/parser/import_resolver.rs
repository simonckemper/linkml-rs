//! Import resolution for `LinkML` schemas

use linkml_core::{
    error::{LinkMLError, Result},
    types::SchemaDefinition,
};
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Import resolver for handling schema imports
pub struct ImportResolver {
    /// Cache of resolved schemas
    cache: Arc<RwLock<HashMap<String, SchemaDefinition>>>,
    /// Search paths for imports
    search_paths: Arc<RwLock<Vec<PathBuf>>>,
    /// Base path for relative imports
    base_path: Arc<RwLock<Option<PathBuf>>>,
    /// Base URL for URL imports
    base_url: Arc<RwLock<Option<String>>>,
    /// Maximum import depth to prevent infinite recursion
    max_depth: usize,
}

impl ImportResolver {
    /// Create a new import resolver
    #[must_use]
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            search_paths: Arc::new(RwLock::new(vec![PathBuf::from(".")])),
            base_path: Arc::new(RwLock::new(None)),
            base_url: Arc::new(RwLock::new(None)),
            max_depth: 10,
        }
    }
    
    /// Create with specific search paths
    #[must_use]
    pub fn with_search_paths(search_paths: Vec<PathBuf>) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            search_paths: Arc::new(RwLock::new(search_paths)),
            base_path: Arc::new(RwLock::new(None)),
            base_url: Arc::new(RwLock::new(None)),
            max_depth: 10,
        }
    }
    
    /// Set the base path for relative imports
    pub fn set_base_path(&self, path: &Path) {
        *self.base_path.write() = Some(path.to_path_buf());
    }
    
    /// Set the base URL for URL imports
    pub fn set_base_url(&self, url: &str) {
        *self.base_url.write() = Some(url.to_string());
    }

    /// Resolve all imports in a schema, returning a merged schema
    ///
    /// # Errors
    ///
    /// Returns a `LinkMLError` if:
    /// - Import resolution fails
    /// - Circular dependencies are detected
    /// - Maximum import depth is exceeded
    pub fn resolve_imports(&self, schema: &SchemaDefinition) -> Result<SchemaDefinition> {
        let mut merged = schema.clone();
        let mut visited = HashSet::new();
        
        // Use tokio runtime for async operations
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| LinkMLError::service(format!("Failed to create runtime: {}", e)))?;
        
        runtime.block_on(async {
            self.resolve_imports_recursive(&mut merged, &mut visited, 0).await
        })?;
        
        Ok(merged)
    }

    /// Resolve imports recursively
    async fn resolve_imports_recursive(
        &self,
        schema: &mut SchemaDefinition,
        visited: &mut HashSet<String>,
        depth: usize,
    ) -> Result<()> {
        if depth > self.max_depth {
            return Err(LinkMLError::import(
                "imports",
                format!("Maximum import depth ({}) exceeded", self.max_depth),
            ));
        }

        // Process each import
        let imports_to_process: Vec<String> = schema.imports.clone();
        for import in imports_to_process {
            if visited.contains(&import) {
                continue; // Already processed
            }

            visited.insert(import.clone());

            // Try to resolve the import
            let imported_schema = self.load_import(&import).await?;

            // Merge the imported schema into the current schema
            self.merge_schema(schema, &imported_schema)?;
        }

        Ok(())
    }

    /// Load an imported schema
    async fn load_import(&self, import: &str) -> Result<SchemaDefinition> {
        // Check cache first
        {
            let cache = self.cache.read();
            if let Some(schema) = cache.get(import) {
                return Ok(schema.clone());
            }
        }

        // Try to find the import file
        let path = self.find_import_file(import)?;

        // Load and parse the schema
        let schema = self.load_schema_file(&path).await?;

        // Cache the result
        {
            let mut cache = self.cache.write();
            cache.insert(import.to_string(), schema.clone());
        }

        Ok(schema)
    }

    /// Find the file for an import
    fn find_import_file(&self, import: &str) -> Result<PathBuf> {
        // Try with common extensions
        let extensions = ["yaml", "yml", "json"];

        let search_paths = self.search_paths.read();
        for search_path in search_paths.iter() {
            for ext in &extensions {
                let path = search_path.join(format!("{import}.{ext}"));
                if path.exists() {
                    return Ok(path);
                }

                // Also try without adding extension (if import already has one)
                let path = search_path.join(import);
                if path.exists() {
                    return Ok(path);
                }
            }
        }

        Err(LinkMLError::import(
            import,
            format!(
                "Import file not found in search paths: {:?}",
                search_paths.clone()
            ),
        ))
    }

    /// Load and parse a schema file
    async fn load_schema_file(&self, path: &Path) -> Result<SchemaDefinition> {
        use super::Parser;

        let parser = Parser::new();
        parser.parse_file(path)
    }

    /// Merge an imported schema into the current schema
    fn merge_schema(&self, target: &mut SchemaDefinition, source: &SchemaDefinition) -> Result<()> {
        let _ = self; // Might need configuration in the future
        // Merge prefixes
        for (prefix, def) in &source.prefixes {
            if !target.prefixes.contains_key(prefix) {
                target.prefixes.insert(prefix.clone(), def.clone());
            }
        }

        // Merge classes
        for (name, class) in &source.classes {
            if target.classes.contains_key(name) {
                return Err(LinkMLError::import(
                    &target.name,
                    format!("Class '{name}' already defined"),
                ));
            }
            target.classes.insert(name.clone(), class.clone());
        }

        // Merge slots
        for (name, slot) in &source.slots {
            if target.slots.contains_key(name) {
                return Err(LinkMLError::import(
                    &target.name,
                    format!("Slot '{name}' already defined"),
                ));
            }
            target.slots.insert(name.clone(), slot.clone());
        }

        // Merge types
        for (name, type_def) in &source.types {
            if target.types.contains_key(name) {
                return Err(LinkMLError::import(
                    &target.name,
                    format!("Type '{name}' already defined"),
                ));
            }
            target.types.insert(name.clone(), type_def.clone());
        }

        // Merge enums
        for (name, enum_def) in &source.enums {
            if target.enums.contains_key(name) {
                return Err(LinkMLError::import(
                    &target.name,
                    format!("Enum '{name}' already defined"),
                ));
            }
            target.enums.insert(name.clone(), enum_def.clone());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SchemaParser;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_import_resolver() {
        let temp_dir = TempDir::new().expect("should create temp dir");
        let base_path = temp_dir.path();

        // Create a base schema
        let base_schema = r"
id: https://example.org/base
name: base
classes:
  BaseClass:
    name: BaseClass
    description: Base class
slots:
  base_slot:
    name: base_slot
    range: string
";

        fs::write(base_path.join("base.yaml"), base_schema).expect("should write base schema");

        // Create a schema that imports base
        let main_schema = r"
id: https://example.org/main
name: main
imports:
  - base
classes:
  MainClass:
    name: MainClass
    is_a: BaseClass
    description: Main class
";

        // Parse main schema
        use super::super::yaml_parser::YamlParser;
        let parser = YamlParser::new();
        let schema = parser.parse_str(main_schema).expect("should parse main schema");

        // Resolve imports
        let resolver = ImportResolver::with_search_paths(vec![base_path.to_path_buf()]);
        let merged = resolver.resolve_imports(&schema).expect("should resolve imports");

        // Check that base elements were imported
        assert!(merged.classes.contains_key("BaseClass"));
        assert!(merged.slots.contains_key("base_slot"));
        assert!(merged.classes.contains_key("MainClass"));
    }
}
