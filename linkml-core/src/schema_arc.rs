//! Arc-based schema handling for efficient sharing
//!
//! This module provides utilities and patterns for working with schemas
//! wrapped in Arc to minimize cloning and improve performance.

use std::sync::Arc;
use std::ops::Deref;

use crate::{
    types::SchemaDefinition,
    error::{LinkMLError, Result},
};

/// Type alias for Arc-wrapped schema
pub type ArcSchema = Arc<SchemaDefinition>;

/// Trait for types that can provide an Arc<SchemaDefinition>
pub trait SchemaProvider {
    /// Get the schema as an Arc
    fn schema(&self) -> &ArcSchema;
    
    /// Get a cloned Arc (cheap operation)
    fn schema_arc(&self) -> ArcSchema {
        Arc::clone(self.schema())
    }
}

/// Wrapper for schema operations that need Arc
pub struct SchemaHandle {
    schema: ArcSchema,
}

impl SchemaHandle {
    /// Create a new schema handle
    pub fn new(schema: SchemaDefinition) -> Self {
        Self {
            schema: Arc::new(schema),
        }
    }
    
    /// Create from existing Arc
    pub fn from_arc(schema: ArcSchema) -> Self {
        Self { schema }
    }
    
    /// Get the inner Arc
    pub fn arc(&self) -> &ArcSchema {
        &self.schema
    }
    
    /// Clone the Arc (cheap operation)
    pub fn clone_arc(&self) -> ArcSchema {
        Arc::clone(&self.schema)
    }
    
    /// Try to get mutable access (only if no other references)
    pub fn try_make_mut(&mut self) -> Option<&mut SchemaDefinition> {
        Arc::get_mut(&mut self.schema)
    }
    
    /// Make a mutable copy if needed (expensive if shared)
    pub fn make_mut(&mut self) -> &mut SchemaDefinition {
        Arc::make_mut(&mut self.schema)
    }
}

impl Deref for SchemaHandle {
    type Target = SchemaDefinition;
    
    fn deref(&self) -> &Self::Target {
        &self.schema
    }
}

impl SchemaProvider for SchemaHandle {
    fn schema(&self) -> &ArcSchema {
        &self.schema
    }
}

/// Builder pattern for creating modified schemas efficiently
pub struct SchemaBuilder {
    base: ArcSchema,
    modifications: SchemaModifications,
}

#[derive(Default)]
struct SchemaModifications {
    name: Option<String>,
    version: Option<String>,
    imports: Option<Vec<String>>,
    // Add more fields as needed
}

impl SchemaBuilder {
    /// Create builder from existing schema
    pub fn from_schema(schema: &ArcSchema) -> Self {
        Self {
            base: Arc::clone(schema),
            modifications: SchemaModifications::default(),
        }
    }
    
    /// Set schema name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.modifications.name = Some(name.into());
        self
    }
    
    /// Set schema version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.modifications.version = Some(version.into());
        self
    }
    
    /// Add imports
    pub fn add_imports(mut self, imports: Vec<String>) -> Self {
        self.modifications.imports = Some(imports);
        self
    }
    
    /// Build the schema (only clones if modifications exist)
    pub fn build(self) -> ArcSchema {
        if self.has_modifications() {
            let mut schema = (*self.base).clone();
            
            if let Some(name) = self.modifications.name {
                schema.name = name;
            }
            if let Some(version) = self.modifications.version {
                schema.version = Some(version);
            }
            if let Some(imports) = self.modifications.imports {
                schema.imports.extend(imports);
            }
            
            Arc::new(schema)
        } else {
            self.base
        }
    }
    
    fn has_modifications(&self) -> bool {
        self.modifications.name.is_some() ||
        self.modifications.version.is_some() ||
        self.modifications.imports.is_some()
    }
}

/// Cache for Arc schemas with interned keys
pub struct SchemaCache {
    cache: dashmap::DashMap<Arc<str>, ArcSchema>,
}

impl SchemaCache {
    /// Create new cache
    pub fn new() -> Self {
        Self {
            cache: dashmap::DashMap::new(),
        }
    }
    
    /// Get or insert schema
    pub fn get_or_insert<F>(&self, key: &str, f: F) -> ArcSchema
    where
        F: FnOnce() -> SchemaDefinition,
    {
        use crate::string_pool::intern;
        let key = intern(key);
        
        self.cache
            .entry(key)
            .or_insert_with(|| Arc::new(f()))
            .clone()
    }
    
    /// Get schema if exists
    pub fn get(&self, key: &str) -> Option<ArcSchema> {
        use crate::string_pool::intern;
        let key = intern(key);
        
        self.cache.get(&key).map(|entry| entry.clone())
    }
    
    /// Insert schema
    pub fn insert(&self, key: &str, schema: ArcSchema) {
        use crate::string_pool::intern;
        let key = intern(key);
        
        self.cache.insert(key, schema);
    }
    
    /// Clear cache
    pub fn clear(&self) {
        self.cache.clear();
    }
}

impl Default for SchemaCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for SchemaDefinition
pub trait SchemaDefinitionExt {
    /// Wrap in Arc
    fn into_arc(self) -> ArcSchema;
}

impl SchemaDefinitionExt for SchemaDefinition {
    fn into_arc(self) -> ArcSchema {
        Arc::new(self)
    }
}

/// Helper to work with multiple schemas efficiently
pub struct SchemaSet {
    schemas: Vec<ArcSchema>,
}

impl SchemaSet {
    /// Create new set
    pub fn new() -> Self {
        Self {
            schemas: Vec::new(),
        }
    }
    
    /// Add schema to set
    pub fn add(&mut self, schema: ArcSchema) {
        self.schemas.push(schema);
    }
    
    /// Iterate over schemas
    pub fn iter(&self) -> impl Iterator<Item = &ArcSchema> {
        self.schemas.iter()
    }
    
    /// Find schema by name
    pub fn find_by_name(&self, name: &str) -> Option<&ArcSchema> {
        self.schemas.iter().find(|s| s.name == name)
    }
    
    /// Merge all schemas into one
    pub fn merge(self) -> Result<ArcSchema> {
        if self.schemas.is_empty() {
            return Err(LinkMLError::internal("Cannot merge empty schema set"));
        }
        
        if self.schemas.len() == 1 {
            return Ok(self.schemas.into_iter().next().unwrap());
        }
        
        // TODO: Implement actual merging logic
        // For now, just clone the first schema
        let mut merged = (*self.schemas[0]).clone();
        merged.name = "merged_schema".to_string();
        
        Ok(Arc::new(merged))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_handle() {
        let schema = SchemaDefinition {
            id: "test".to_string(),
            name: "test".to_string(),
            ..Default::default()
        };
        
        let handle = SchemaHandle::new(schema);
        assert_eq!(handle.name, "test");
        
        let arc1 = handle.clone_arc();
        let arc2 = handle.clone_arc();
        assert!(Arc::ptr_eq(&arc1, &arc2));
    }

    #[test]
    fn test_schema_builder() {
        let original = Arc::new(SchemaDefinition {
            id: "test".to_string(),
            name: "original".to_string(),
            ..Default::default()
        });
        
        // No modifications - should return same Arc
        let same = SchemaBuilder::from_schema(&original).build();
        assert!(Arc::ptr_eq(&original, &same));
        
        // With modifications - should create new Arc
        let modified = SchemaBuilder::from_schema(&original)
            .with_name("modified")
            .build();
        assert!(!Arc::ptr_eq(&original, &modified));
        assert_eq!(modified.name, "modified");
    }

    #[test]
    fn test_schema_cache() {
        let cache = SchemaCache::new();
        
        let schema1 = cache.get_or_insert("test", || SchemaDefinition {
            id: "test".to_string(),
            name: "test".to_string(),
            ..Default::default()
        });
        
        let schema2 = cache.get("test").unwrap();
        assert!(Arc::ptr_eq(&schema1, &schema2));
    }
}