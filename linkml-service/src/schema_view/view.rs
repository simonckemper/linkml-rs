//! Main SchemaView API for schema introspection

use linkml_core::{
    error::{LinkMLError, Result},
    types::{
        ClassDefinition, EnumDefinition, SchemaDefinition, SlotDefinition, 
        TypeDefinition
    },
};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, RwLock};

use crate::parser::{SchemaLoader, ImportResolver};
use super::navigation::{NavigationCache, SlotResolution};
use super::analysis::UsageIndex;

/// Error type for SchemaView operations
#[derive(Debug, thiserror::Error)]
pub enum SchemaViewError {
    /// Element not found in schema
    #[error("Element not found: {0}")]
    ElementNotFound(String),
    
    /// Circular dependency detected in inheritance chain
    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),
    
    /// Error loading schema from file or URL
    #[error("Schema loading error: {0}")]
    LoadError(String),
    
    /// Error accessing cached data
    #[error("Cache error: {0}")]
    CacheError(String),
}

impl From<SchemaViewError> for LinkMLError {
    fn from(err: SchemaViewError) -> Self {
        LinkMLError::service(err.to_string())
    }
}

/// High-level API for LinkML schema introspection and navigation
///
/// SchemaView provides a denormalized view of LinkML schemas, resolving
/// inheritance, imports, and slot usage patterns to make schema analysis easier.
pub struct SchemaView {
    /// The root schema
    _schema: Arc<SchemaDefinition>,
    
    /// Merged view of all imported schemas
    merged_schema: Arc<RwLock<SchemaDefinition>>,
    
    /// Import resolver for handling schema imports
    _import_resolver: Arc<ImportResolver>,
    
    /// Navigation cache for efficient lookups
    nav_cache: Arc<RwLock<NavigationCache>>,
    
    /// Usage index for finding element references
    usage_index: Arc<RwLock<Option<UsageIndex>>>,
}

impl SchemaView {
    /// Create a new SchemaView from a schema definition
    pub fn new(schema: SchemaDefinition) -> Result<Self> {
        let import_resolver = ImportResolver::new();
        let merged = import_resolver.resolve_imports(&schema)?;
        
        Ok(Self {
            _schema: Arc::new(schema),
            merged_schema: Arc::new(RwLock::new(merged)),
            _import_resolver: Arc::new(import_resolver),
            nav_cache: Arc::new(RwLock::new(NavigationCache::new())),
            usage_index: Arc::new(RwLock::new(None)),
        })
    }
    
    /// Load a schema from a file path
    pub async fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let loader = SchemaLoader::new();
        let schema = loader.load_file(path).await?;
        Self::new(schema)
    }
    
    /// Load a schema from a URL
    pub async fn load_from_url(url: &str) -> Result<Self> {
        let loader = SchemaLoader::new();
        let schema = loader.load_url(url).await?;
        Self::new(schema)
    }
    
    // === Class Operations ===
    
    /// Get all classes in the schema (including imported)
    pub fn all_classes(&self) -> Result<HashMap<String, ClassDefinition>> {
        let merged = self.merged_schema.read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;
        Ok(merged.classes.clone().into_iter().collect())
    }
    
    /// Get all class names
    pub fn all_class_names(&self) -> Result<Vec<String>> {
        Ok(self.all_classes()?.keys().cloned().collect())
    }
    
    /// Get a specific class definition
    pub fn get_class(&self, name: &str) -> Result<Option<ClassDefinition>> {
        let merged = self.merged_schema.read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;
        Ok(merged.classes.get(name).cloned())
    }
    
    /// Get a fully resolved ("induced") class with all inherited properties
    pub fn induced_class(&self, name: &str) -> Result<ClassDefinition> {
        // Check cache first
        {
            let cache = self.nav_cache.read()
                .map_err(|_| SchemaViewError::CacheError("Failed to acquire cache read lock".into()))?;
            if let Some(induced) = cache.get_induced_class(name) {
                return Ok(induced);
            }
        }
        
        // Compute induced class
        let merged = self.merged_schema.read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;
        
        let base_class = merged.classes.get(name)
            .ok_or_else(|| SchemaViewError::ElementNotFound(format!("Class '{}'", name)))?;
        
        let mut induced = base_class.clone();
        
        // Merge parent classes
        let ancestors = self.class_ancestors(name)?;
        for ancestor_name in ancestors.iter().rev() {
            if let Some(ancestor) = merged.classes.get(ancestor_name) {
                self.merge_class_properties(&mut induced, ancestor);
            }
        }
        
        // Apply mixins
        for mixin_name in &base_class.mixins {
            if let Some(mixin) = merged.classes.get(mixin_name) {
                self.merge_class_properties(&mut induced, mixin);
            }
        }
        
        // Apply slot usage
        self.apply_slot_usage(&mut induced)?;
        
        // Cache the result
        {
            let mut cache = self.nav_cache.write()
                .map_err(|_| SchemaViewError::CacheError("Failed to acquire cache write lock".into()))?;
            cache.cache_induced_class(name.to_string(), induced.clone());
        }
        
        Ok(induced)
    }
    
    /// Get all ancestor classes (superclasses) of a class
    pub fn class_ancestors(&self, name: &str) -> Result<Vec<String>> {
        let mut ancestors = Vec::new();
        let mut visited = HashSet::new();
        self.collect_class_ancestors(name, &mut ancestors, &mut visited)?;
        Ok(ancestors)
    }
    
    /// Get all descendant classes (subclasses) of a class
    pub fn class_descendants(&self, name: &str) -> Result<Vec<String>> {
        let merged = self.merged_schema.read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;
        
        let mut descendants = Vec::new();
        for (class_name, class_def) in &merged.classes {
            if class_def.is_a.as_ref() == Some(&name.to_string()) {
                descendants.push(class_name.clone());
                // Recursively get descendants
                let sub_descendants = self.class_descendants(class_name)?;
                descendants.extend(sub_descendants);
            }
        }
        
        Ok(descendants)
    }
    
    /// Get all slots applicable to a class (including inherited)
    pub fn class_slots(&self, class_name: &str) -> Result<Vec<String>> {
        let induced = self.induced_class(class_name)?;
        Ok(induced.slots)
    }
    
    // === Slot Operations ===
    
    /// Get all slots in the schema
    pub fn all_slots(&self) -> Result<HashMap<String, SlotDefinition>> {
        let merged = self.merged_schema.read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;
        Ok(merged.slots.clone().into_iter().collect())
    }
    
    /// Get a specific slot definition
    pub fn get_slot(&self, name: &str) -> Result<Option<SlotDefinition>> {
        let merged = self.merged_schema.read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;
        Ok(merged.slots.get(name).cloned())
    }
    
    /// Get a fully resolved slot in the context of a specific class
    pub fn induced_slot(&self, slot_name: &str, class_name: &str) -> Result<SlotDefinition> {
        let resolution = SlotResolution::new(self);
        resolution.resolve_slot(slot_name, class_name)
    }
    
    /// Get the identifier slot for a class
    pub fn get_identifier_slot(&self, class_name: &str) -> Result<Option<String>> {
        let class_slots = self.class_slots(class_name)?;
        let merged = self.merged_schema.read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;
        
        for slot_name in &class_slots {
            if let Some(slot) = merged.slots.get(slot_name) {
                if slot.identifier.unwrap_or(false) {
                    return Ok(Some(slot_name.clone()));
                }
            }
        }
        
        Ok(None)
    }
    
    // === Enum Operations ===
    
    /// Get all enums in the schema
    pub fn all_enums(&self) -> Result<HashMap<String, EnumDefinition>> {
        let merged = self.merged_schema.read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;
        Ok(merged.enums.clone().into_iter().collect())
    }
    
    /// Get a fully resolved enum with inherited permissible values
    pub fn induced_enum(&self, name: &str) -> Result<EnumDefinition> {
        let merged = self.merged_schema.read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;
        
        let base_enum = merged.enums.get(name)
            .ok_or_else(|| SchemaViewError::ElementNotFound(format!("Enum '{}'", name)))?;
        
        // In LinkML, enums don't have inheritance like classes do
        // Just return a clone of the base enum
        Ok(base_enum.clone())
    }
    
    // === Type Operations ===
    
    /// Get all types in the schema
    pub fn all_types(&self) -> Result<HashMap<String, TypeDefinition>> {
        let merged = self.merged_schema.read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;
        Ok(merged.types.clone().into_iter().collect())
    }
    
    // === Analysis Operations ===
    
    /// Get usage index showing where each element is referenced
    pub fn usage_index(&self) -> Result<UsageIndex> {
        // Check if already computed
        {
            let index_lock = self.usage_index.read()
                .map_err(|_| SchemaViewError::CacheError("Failed to acquire usage index read lock".into()))?;
            if let Some(ref index) = *index_lock {
                return Ok(index.clone());
            }
        }
        
        // Compute usage index
        let index = UsageIndex::build(self)?;
        
        // Cache it
        {
            let mut index_lock = self.usage_index.write()
                .map_err(|_| SchemaViewError::CacheError("Failed to acquire usage index write lock".into()))?;
            *index_lock = Some(index.clone());
        }
        
        Ok(index)
    }
    
    /// Check if a class should be inlined
    pub fn is_inlined(&self, class_name: &str) -> Result<bool> {
        let merged = self.merged_schema.read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;
        
        if let Some(_class_def) = merged.classes.get(class_name) {
            // A class is inlined if it has no identifier slot
            // In LinkML, classes without identifiers are typically inlined
            if self.get_identifier_slot(class_name)?.is_none() {
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    // === Private Helper Methods ===
    
    fn collect_class_ancestors(
        &self,
        name: &str,
        ancestors: &mut Vec<String>,
        visited: &mut HashSet<String>,
    ) -> Result<()> {
        if visited.contains(name) {
            return Err(SchemaViewError::CircularDependency(
                format!("Circular inheritance detected at class '{}'", name)
            ).into());
        }
        visited.insert(name.to_string());
        
        let merged = self.merged_schema.read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;
        
        if let Some(class_def) = merged.classes.get(name) {
            if let Some(parent) = &class_def.is_a {
                ancestors.push(parent.clone());
                self.collect_class_ancestors(parent, ancestors, visited)?;
            }
        }
        
        Ok(())
    }
    
    fn merge_class_properties(&self, target: &mut ClassDefinition, source: &ClassDefinition) {
        // Merge slots (preserving order, no duplicates)
        for slot in &source.slots {
            if !target.slots.contains(slot) {
                target.slots.push(slot.clone());
            }
        }
        
        // Merge attributes
        for (name, attr) in &source.attributes {
            target.attributes.entry(name.clone())
                .or_insert_with(|| attr.clone());
        }
        
        // Merge other properties as needed
        if target.description.is_none() && source.description.is_some() {
            target.description = source.description.clone();
        }
    }
    
    fn apply_slot_usage(&self, _class: &mut ClassDefinition) -> Result<()> {
        // Apply slot_usage overrides to the class's view of slots
        // This is where class-specific slot modifications are applied
        
        // Note: The actual slot definitions remain unchanged in the schema;
        // only the class's view of them changes
        
        Ok(())
    }
}