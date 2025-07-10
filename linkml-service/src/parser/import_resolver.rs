//! Import resolution for LinkML schemas

use linkml_core::{
    error::{LinkMLError, Result},
    types::SchemaDefinition,
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;

/// Import resolver for handling schema imports
pub struct ImportResolver {
    /// Cache of resolved schemas
    cache: Arc<RwLock<HashMap<String, SchemaDefinition>>>,
    /// Search paths for imports
    search_paths: Vec<PathBuf>,
    /// Maximum import depth to prevent infinite recursion
    max_depth: usize,
}

impl ImportResolver {
    /// Create a new import resolver
    pub fn new(search_paths: Vec<PathBuf>) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            search_paths,
            max_depth: 10,
        }
    }
    
    /// Resolve all imports in a schema
    pub async fn resolve_imports(&self, schema: &mut SchemaDefinition) -> Result<()> {
        let mut visited = HashSet::new();
        self.resolve_imports_recursive(schema, &mut visited, 0).await
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
                format!("Maximum import depth ({}) exceeded", self.max_depth)
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
        
        for search_path in &self.search_paths {
            for ext in &extensions {
                let path = search_path.join(format!("{}.{}", import, ext));
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
            format!("Import file not found in search paths: {:?}", self.search_paths)
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
                    format!("Class '{}' already defined", name)
                ));
            }
            target.classes.insert(name.clone(), class.clone());
        }
        
        // Merge slots
        for (name, slot) in &source.slots {
            if target.slots.contains_key(name) {
                return Err(LinkMLError::import(
                    &target.name,
                    format!("Slot '{}' already defined", name)
                ));
            }
            target.slots.insert(name.clone(), slot.clone());
        }
        
        // Merge types
        for (name, type_def) in &source.types {
            if target.types.contains_key(name) {
                return Err(LinkMLError::import(
                    &target.name,
                    format!("Type '{}' already defined", name)
                ));
            }
            target.types.insert(name.clone(), type_def.clone());
        }
        
        // Merge enums
        for (name, enum_def) in &source.enums {
            if target.enums.contains_key(name) {
                return Err(LinkMLError::import(
                    &target.name,
                    format!("Enum '{}' already defined", name)
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
    use tempfile::TempDir;
    use std::fs;
    use crate::parser::SchemaParser;
    
    #[tokio::test]
    async fn test_import_resolver() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();
        
        // Create a base schema
        let base_schema = r#"
id: https://example.org/base
name: base
classes:
  BaseClass:
    description: Base class
slots:
  base_slot:
    range: string
"#;
        
        fs::write(base_path.join("base.yaml"), base_schema).unwrap();
        
        // Create a schema that imports base
        let main_schema = r#"
id: https://example.org/main
name: main
imports:
  - base
classes:
  MainClass:
    is_a: BaseClass
    description: Main class
"#;
        
        // Parse main schema
        use super::super::yaml_parser::YamlParser;
        let parser = YamlParser::new();
        let mut schema = parser.parse_str(main_schema).unwrap();
        
        // Resolve imports
        let resolver = ImportResolver::new(vec![base_path.to_path_buf()]);
        resolver.resolve_imports(&mut schema).await.unwrap();
        
        // Check that base elements were imported
        assert!(schema.classes.contains_key("BaseClass"));
        assert!(schema.slots.contains_key("base_slot"));
        assert!(schema.classes.contains_key("MainClass"));
    }
}