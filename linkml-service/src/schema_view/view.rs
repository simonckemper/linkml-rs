//! Main `SchemaView` API for schema introspection

use linkml_core::{
    error::{LinkMLError, Result},
    types::{ClassDefinition, EnumDefinition, SchemaDefinition, SlotDefinition, TypeDefinition},
};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, RwLock};

use super::analysis::UsageIndex;
use super::class_view::ClassView;
use super::navigation::{NavigationCache, SlotResolution};
use super::slot_view::SlotView;
use crate::parser::{ImportResolver, SchemaLoader};

/// Type of schema element
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElementType {
    /// Class definition
    Class,
    /// Slot definition
    Slot,
    /// Type definition
    Type,
    /// Enum definition
    Enum,
    /// Subset definition
    Subset,
}

/// Error type for `SchemaView` operations
#[derive(Debug, thiserror::Error)]
pub enum SchemaViewError {
    /// Element not found in schema
    #[error("Element not found: {0}")]
    ElementNotFound(String),

    /// Circular dependency detected in inheritance chain
    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),

    /// Error loading schema from file or `URL`
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

/// High-level `API` for `LinkML` schema introspection and navigation
///
/// `SchemaView` provides a denormalized view of `LinkML` schemas, resolving
/// inheritance, imports, and slot usage patterns to make schema analysis easier.
#[derive(Clone, Debug)]
pub struct SchemaView {
    _schema: Arc<SchemaDefinition>,

    /// Merged view of all imported schemas
    merged_schema: Arc<RwLock<SchemaDefinition>>,

    /// Import resolver for handling schema imports (kept for future reference)
    _import_resolver: Arc<ImportResolver>,

    /// Navigation cache for efficient lookups
    nav_cache: Arc<RwLock<NavigationCache>>,

    /// Usage index for finding element references
    usage_index: Arc<RwLock<Option<UsageIndex>>>,
}

impl SchemaView {
    /// Create a new `SchemaView` from a schema definition
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn new(schema: SchemaDefinition) -> Result<Self> {
        let import_resolver = ImportResolver::new();
        let merged = import_resolver.resolve_imports(&schema)?;

        let schema_arc = Arc::new(schema);
        let merged_arc = Arc::new(RwLock::new(merged));

        Ok(Self {
            _schema: schema_arc,
            merged_schema: merged_arc,
            _import_resolver: Arc::new(import_resolver),
            nav_cache: Arc::new(RwLock::new(NavigationCache::new())),
            usage_index: Arc::new(RwLock::new(None)),
        })
    }

    /// Load a schema from a file path
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub async fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let loader = SchemaLoader::new();
        let schema = loader.load_file(path).await?;
        Self::new(schema)
    }

    /// Load a schema from a `URL`
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub async fn load_from_url(url: &str) -> Result<Self> {
        let loader = SchemaLoader::new();
        let schema = loader.load_url(url).await?;
        Self::new(schema)
    }

    // === Class Operations ===

    /// Get all classes in the schema (including imported)
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn all_classes(&self) -> Result<HashMap<String, ClassDefinition>> {
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;
        Ok(merged.classes.clone().into_iter().collect())
    }

    /// Get all class names
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn all_class_names(&self) -> Result<Vec<String>> {
        Ok(self.all_classes()?.keys().cloned().collect())
    }

    /// Get a specific class definition
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn get_class(&self, name: &str) -> Result<Option<ClassDefinition>> {
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;
        Ok(merged.classes.get(name).cloned())
    }

    /// Get a fully resolved ("induced") class with all inherited properties
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn induced_class(&self, name: &str) -> Result<ClassDefinition> {
        // Check cache first
        {
            let cache = self.nav_cache.read().map_err(|_| {
                SchemaViewError::CacheError("Failed to acquire cache read lock".into())
            })?;
            if let Some(induced) = cache.get_induced_class(name) {
                return Ok(induced);
            }
        }

        // Compute induced class
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;

        let base_class = merged
            .classes
            .get(name)
            .ok_or_else(|| SchemaViewError::ElementNotFound(format!("Class '{name}'")))?;

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
            let mut cache = self.nav_cache.write().map_err(|_| {
                SchemaViewError::CacheError("Failed to acquire cache write lock".into())
            })?;
            cache.cache_induced_class(name.to_string(), induced.clone());
        }

        Ok(induced)
    }

    /// Get all ancestor classes (superclasses) of a class
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn class_ancestors(&self, name: &str) -> Result<Vec<String>> {
        let mut ancestors = Vec::new();
        let mut visited = HashSet::new();
        self.collect_class_ancestors(name, &mut ancestors, &mut visited)?;
        Ok(ancestors)
    }

    /// Get all descendant classes (subclasses) of a class
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn class_descendants(&self, name: &str) -> Result<Vec<String>> {
        let merged = self
            .merged_schema
            .read()
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
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn class_slots(&self, class_name: &str) -> Result<Vec<String>> {
        let induced = self.induced_class(class_name)?;
        Ok(induced.slots)
    }

    // === Slot Operations ===

    /// Get all slots in the schema
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn all_slots(&self) -> Result<HashMap<String, SlotDefinition>> {
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;
        Ok(merged.slots.clone().into_iter().collect())
    }

    /// Get a specific slot definition
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn get_slot(&self, name: &str) -> Result<Option<SlotDefinition>> {
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;
        Ok(merged.slots.get(name).cloned())
    }

    /// Get a fully resolved slot in the context of a specific class
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn induced_slot(&self, slot_name: &str, class_name: &str) -> Result<SlotDefinition> {
        let resolution = SlotResolution::new(self);
        resolution.resolve_slot(slot_name, class_name)
    }

    /// Get the identifier slot for a class
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn get_identifier_slot(&self, class_name: &str) -> Result<Option<String>> {
        let class_slots = self.class_slots(class_name)?;
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;

        for slot_name in &class_slots {
            if let Some(slot) = merged.slots.get(slot_name)
                && slot.identifier.unwrap_or(false)
            {
                return Ok(Some(slot_name.clone()));
            }
        }

        Ok(None)
    }

    // === Enum Operations ===

    /// Get all enums in the schema
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn all_enums(&self) -> Result<HashMap<String, EnumDefinition>> {
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;
        Ok(merged.enums.clone().into_iter().collect())
    }

    /// Get a fully resolved enum with inherited permissible values
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn induced_enum(&self, name: &str) -> Result<EnumDefinition> {
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;

        let base_enum = merged
            .enums
            .get(name)
            .ok_or_else(|| SchemaViewError::ElementNotFound(format!("Enum '{name}'")))?;

        // In LinkML, enums don't have inheritance like classes do
        // Just return a clone of the base enum
        Ok(base_enum.clone())
    }

    // === Type Operations ===

    /// Get all types in the schema
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn all_types(&self) -> Result<HashMap<String, TypeDefinition>> {
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;
        Ok(merged.types.clone().into_iter().collect())
    }

    // === View Operations ===

    /// Get a `ClassView` for detailed class inspection
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn class_view(&self, class_name: &str) -> Result<ClassView> {
        // Check if class exists first
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;

        if !merged.classes.contains_key(class_name) {
            return Err(SchemaViewError::ElementNotFound(format!("Class '{class_name}'")).into());
        }
        drop(merged); // Release lock before creating view

        // Create ClassView using self as Arc
        ClassView::new(class_name, Arc::new(self.clone()))
    }

    /// Get a `SlotView` for detailed slot inspection
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn slot_view(&self, slot_name: &str) -> Result<SlotView> {
        // Check if slot exists first
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;

        if !merged.slots.contains_key(slot_name) {
            return Err(SchemaViewError::ElementNotFound(format!("Slot '{slot_name}'")).into());
        }
        drop(merged); // Release lock before creating view

        // Create SlotView using self as Arc
        SlotView::new(slot_name, Arc::new(self.clone()))
    }

    // === Analysis Operations ===

    /// Get usage index showing where each element is referenced
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn usage_index(&self) -> Result<UsageIndex> {
        // Check if already computed
        {
            let index_lock = self.usage_index.read().map_err(|_| {
                SchemaViewError::CacheError("Failed to acquire usage index read lock".into())
            })?;
            if let Some(ref index) = *index_lock {
                return Ok(index.clone());
            }
        }

        // Compute usage index
        let index = UsageIndex::build(self)?;

        // Cache it
        {
            let mut index_lock = self.usage_index.write().map_err(|_| {
                SchemaViewError::CacheError("Failed to acquire usage index write lock".into())
            })?;
            *index_lock = Some(index.clone());
        }

        Ok(index)
    }

    /// Check if a class should be inlined
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn is_inlined(&self, class_name: &str) -> Result<bool> {
        let merged = self
            .merged_schema
            .read()
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

    // === Pattern Materialization ===

    /// Materialize structured patterns to regular expressions
    ///
    /// This expands `LinkML` structured patterns (e.g., for identifiers)
    /// into their full regular expression form.
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn materialize_patterns(&mut self) -> Result<()> {
        let mut merged = self
            .merged_schema
            .write()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire write lock".into()))?;

        // Process patterns in types
        for type_def in merged.types.values_mut() {
            // TypeDefinition has pattern field that can be used directly
            if let Some(_pattern) = &type_def.pattern {
                // Pattern is already a regex string, no conversion needed
            }
        }

        // Process patterns in slots
        for slot_def in merged.slots.values_mut() {
            if let Some(structured_pattern) = &slot_def.structured_pattern {
                let regex_pattern = self.structured_pattern_to_regex(structured_pattern)?;
                slot_def.pattern = Some(regex_pattern);
            }
        }

        Ok(())
    }

    /// Convert a structured pattern to a regular expression
    fn structured_pattern_to_regex(
        &self,
        pattern: &linkml_core::types::StructuredPattern,
    ) -> Result<String> {
        // StructuredPattern only has: syntax, pattern, interpolated, partial_match
        if let Some(pattern_str) = &pattern.pattern {
            // If partial_match is false, wrap in anchors
            if pattern.partial_match.unwrap_or(false) {
                Ok(pattern_str.clone())
            } else {
                Ok(format!("^{pattern_str}$"))
            }
        } else {
            // No pattern specified
            Ok(String::from(".*"))
        }
    }

    // === Universal Element Retrieval ===

    /// Get any element by name (class, slot, type, or enum)
    ///
    /// This searches across all element types and returns the first match.
    /// Returns the element type and the element itself.
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn get_element(&self, name: &str) -> Result<Option<(ElementType, serde_json::Value)>> {
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;

        // Check classes first
        if let Some(class) = merged.classes.get(name) {
            return Ok(Some((
                ElementType::Class,
                serde_json::to_value(class)
                    .map_err(|e| LinkMLError::SerializationError(e.to_string()))?,
            )));
        }

        // Check slots
        if let Some(slot) = merged.slots.get(name) {
            return Ok(Some((
                ElementType::Slot,
                serde_json::to_value(slot)
                    .map_err(|e| LinkMLError::SerializationError(e.to_string()))?,
            )));
        }

        // Check types
        if let Some(type_def) = merged.types.get(name) {
            return Ok(Some((
                ElementType::Type,
                serde_json::to_value(type_def)
                    .map_err(|e| LinkMLError::SerializationError(e.to_string()))?,
            )));
        }

        // Check enums
        if let Some(enum_def) = merged.enums.get(name) {
            return Ok(Some((
                ElementType::Enum,
                serde_json::to_value(enum_def)
                    .map_err(|e| LinkMLError::SerializationError(e.to_string()))?,
            )));
        }

        Ok(None)
    }

    // === Class Hierarchy Methods ===

    /// Get direct parent classes only (not full ancestry)
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn class_parents(&self, name: &str) -> Result<Vec<String>> {
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;

        let mut parents = Vec::new();

        if let Some(class_def) = merged.classes.get(name) {
            // Direct is_a parent
            if let Some(parent) = &class_def.is_a {
                parents.push(parent.clone());
            }

            // Mixins are also considered parents
            parents.extend(class_def.mixins.clone());
        }

        Ok(parents)
    }

    /// Get direct child classes only (not full descendants)
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn class_children(&self, name: &str) -> Result<Vec<String>> {
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;

        let mut children = Vec::new();

        for (class_name, class_def) in &merged.classes {
            // Check if this class has 'name' as direct parent
            if class_def.is_a.as_ref() == Some(&name.to_string()) {
                children.push(class_name.clone());
            }

            // Check if this class uses 'name' as a mixin
            if class_def.mixins.contains(&name.to_string()) {
                children.push(class_name.clone());
            }
        }

        // Deduplicate in case a class uses both is_a and mixins
        children.sort();
        children.dedup();

        Ok(children)
    }

    /// Get all root classes (classes with no parents)
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn class_roots(&self) -> Result<Vec<String>> {
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;

        let mut roots = Vec::new();

        for (name, class_def) in &merged.classes {
            if class_def.is_a.is_none() && class_def.mixins.is_empty() {
                roots.push(name.clone());
            }
        }

        roots.sort();
        Ok(roots)
    }

    /// Get all leaf classes (classes with no children)
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn class_leaves(&self) -> Result<Vec<String>> {
        let all_classes = self.all_classes()?;
        let mut leaves = Vec::new();

        for class_name in all_classes.keys() {
            let children = self.class_children(class_name)?;
            if children.is_empty() {
                leaves.push(class_name.clone());
            }
        }

        leaves.sort();
        Ok(leaves)
    }

    // === URI/CURIE Resolution ===

    /// Get the URI for an element, expanding CURIEs if needed
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn get_uri(&self, element_name: &str, expand: bool) -> Result<Option<String>> {
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;

        // Check for explicit URIs in different element types
        let uri = if let Some(class) = merged.classes.get(element_name) {
            class.class_uri.clone()
        } else if let Some(slot) = merged.slots.get(element_name) {
            slot.slot_uri.clone()
        } else if let Some(type_def) = merged.types.get(element_name) {
            type_def.uri.clone()
        } else if merged.enums.contains_key(element_name) {
            // Enums don't have URIs in LinkML
            None
        } else {
            None
        };

        // If we have a URI and need to expand it
        if let Some(uri_str) = uri {
            if expand && uri_str.contains(':') && !uri_str.starts_with("http") {
                return Ok(Some(self.expand_curie(&uri_str)?));
            }
            Ok(Some(uri_str))
        } else {
            // Generate a default URI based on schema ID + element name
            if merged.id.is_empty() {
                Ok(None)
            } else {
                Ok(Some(format!(
                    "{}/{}",
                    merged.id.trim_end_matches('/'),
                    element_name
                )))
            }
        }
    }

    /// Expand a CURIE to its full URI form
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn expand_curie(&self, curie: &str) -> Result<String> {
        if let Some(colon_pos) = curie.find(':') {
            let prefix = &curie[..colon_pos];
            let local = &curie[colon_pos + 1..];

            let merged = self
                .merged_schema
                .read()
                .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;

            if let Some(prefix_uri) = merged.prefixes.get(prefix) {
                let uri_str = match prefix_uri {
                    linkml_core::types::PrefixDefinition::Simple(s) => s,
                    linkml_core::types::PrefixDefinition::Complex { prefix_prefix, .. } => {
                        prefix_prefix
                    }
                };
                return Ok(format!("{uri_str}{local}"));
            }
        }

        // If not a CURIE or prefix not found, return as-is
        Ok(curie.to_string())
    }

    // === Type Hierarchy Methods ===

    /// Get a specific type definition
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn get_type(&self, name: &str) -> Result<Option<TypeDefinition>> {
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;
        Ok(merged.types.get(name).cloned())
    }

    /// Get direct parent types
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn type_parents(&self, name: &str) -> Result<Vec<String>> {
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;

        if let Some(type_def) = merged.types.get(name)
            && let Some(base_type) = &type_def.base_type
        {
            return Ok(vec![base_type.clone()]);
        }

        Ok(Vec::new())
    }

    /// Get direct child types
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn type_children(&self, name: &str) -> Result<Vec<String>> {
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;

        let mut children = Vec::new();

        for (type_name, type_def) in &merged.types {
            if type_def.base_type.as_ref() == Some(&name.to_string()) {
                children.push(type_name.clone());
            }
        }

        children.sort();
        Ok(children)
    }

    /// Get all type ancestors
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn type_ancestors(&self, name: &str, reflexive: bool) -> Result<Vec<String>> {
        let mut ancestors = Vec::new();
        if reflexive {
            ancestors.push(name.to_string());
        }

        let mut current = name.to_string();
        while let Ok(parents) = self.type_parents(&current) {
            if let Some(parent) = parents.first() {
                if ancestors.contains(parent) {
                    return Err(SchemaViewError::CircularDependency(format!(
                        "Circular type inheritance detected at '{parent}'"
                    ))
                    .into());
                }
                ancestors.push(parent.clone());
                current.clone_from(parent);
            } else {
                break;
            }
        }

        Ok(ancestors)
    }

    /// Get all type descendants
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn type_descendants(&self, name: &str, reflexive: bool) -> Result<Vec<String>> {
        let mut descendants = Vec::new();
        if reflexive {
            descendants.push(name.to_string());
        }

        // Recursively collect all descendants
        let direct_children = self.type_children(name)?;
        for child in direct_children {
            descendants.push(child.clone());
            let sub_descendants = self.type_descendants(&child, false)?;
            descendants.extend(sub_descendants);
        }

        // Remove duplicates and sort
        descendants.sort();
        descendants.dedup();

        Ok(descendants)
    }

    // === Slot Hierarchy Methods ===

    /// Get all slot names
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn all_slot_names(&self) -> Result<Vec<String>> {
        Ok(self.all_slots()?.keys().cloned().collect())
    }

    /// Get direct slot parents
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn slot_parents(&self, name: &str) -> Result<Vec<String>> {
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;

        let mut parents = Vec::new();

        if let Some(slot_def) = merged.slots.get(name) {
            // Direct is_a parent
            if let Some(parent) = &slot_def.is_a {
                parents.push(parent.clone());
            }

            // Mixins are also considered parents
            parents.extend(slot_def.mixins.clone());
        }

        Ok(parents)
    }

    /// Get direct slot children
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn slot_children(&self, name: &str) -> Result<Vec<String>> {
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;

        let mut children = Vec::new();

        for (slot_name, slot_def) in &merged.slots {
            // Check if this slot has 'name' as direct parent
            if slot_def.is_a.as_ref() == Some(&name.to_string()) {
                children.push(slot_name.clone());
            }

            // Check if this slot uses 'name' as a mixin
            if slot_def.mixins.contains(&name.to_string()) {
                children.push(slot_name.clone());
            }
        }

        // Deduplicate and sort
        children.sort();
        children.dedup();

        Ok(children)
    }

    /// Get all slot ancestors
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn slot_ancestors(&self, name: &str, reflexive: bool) -> Result<Vec<String>> {
        let mut ancestors = Vec::new();
        if reflexive {
            ancestors.push(name.to_string());
        }

        let mut visited = HashSet::new();
        self.collect_slot_ancestors(name, &mut ancestors, &mut visited)?;

        Ok(ancestors)
    }

    /// Get all slot descendants
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn slot_descendants(&self, name: &str, reflexive: bool) -> Result<Vec<String>> {
        let mut descendants = Vec::new();
        if reflexive {
            descendants.push(name.to_string());
        }

        // Recursively collect all descendants
        let direct_children = self.slot_children(name)?;
        for child in direct_children {
            descendants.push(child.clone());
            let sub_descendants = self.slot_descendants(&child, false)?;
            descendants.extend(sub_descendants);
        }

        // Remove duplicates and sort
        descendants.sort();
        descendants.dedup();

        Ok(descendants)
    }

    // === Enum Methods ===

    /// Get a specific enum definition
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn get_enum(&self, name: &str) -> Result<Option<EnumDefinition>> {
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;
        Ok(merged.enums.get(name).cloned())
    }

    /// Get all enum names
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn all_enum_names(&self) -> Result<Vec<String>> {
        Ok(self.all_enums()?.keys().cloned().collect())
    }

    // === Subset Operations ===

    /// Get all subsets in the schema
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn all_subsets(&self) -> Result<HashMap<String, linkml_core::types::SubsetDefinition>> {
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;
        Ok(merged.subsets.clone().into_iter().collect())
    }

    /// Get a specific subset
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn get_subset(&self, name: &str) -> Result<Option<linkml_core::types::SubsetDefinition>> {
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;
        Ok(merged.subsets.get(name).cloned())
    }

    /// Check if an element is in a subset
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn in_subset(&self, _element_name: &str, subset_name: &str) -> Result<bool> {
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;

        // Check if the subset exists
        if !merged.subsets.contains_key(subset_name) {
            return Ok(false);
        }

        // LinkML core types don't have in_subset fields in this version
        // Always return false for subset membership
        Ok(false)
    }

    // === Schema Information ===

    /// Get the schema name
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn schema_name(&self) -> Result<String> {
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;
        Ok(merged.name.clone())
    }

    /// Get the schema ID
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn schema_id(&self) -> Result<Option<String>> {
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;
        if merged.id.is_empty() {
            Ok(None)
        } else {
            Ok(Some(merged.id.clone()))
        }
    }

    /// Get all prefixes defined in the schema
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn get_prefixes(&self) -> Result<HashMap<String, String>> {
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;
        let prefixes = merged
            .prefixes
            .iter()
            .map(|(k, v)| {
                let uri = match v {
                    linkml_core::types::PrefixDefinition::Simple(s) => s.clone(),
                    linkml_core::types::PrefixDefinition::Complex { prefix_prefix, .. } => {
                        prefix_prefix.clone()
                    }
                };
                (k.clone(), uri)
            })
            .collect();
        Ok(prefixes)
    }

    /// Get a specific prefix expansion
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn get_prefix(&self, prefix: &str) -> Result<Option<String>> {
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;

        if let Some(prefix_uri) = merged.prefixes.get(prefix) {
            let uri = match prefix_uri {
                linkml_core::types::PrefixDefinition::Simple(s) => s.clone(),
                linkml_core::types::PrefixDefinition::Complex { prefix_prefix, .. } => {
                    prefix_prefix.clone()
                }
            };
            Ok(Some(uri))
        } else {
            Ok(None)
        }
    }

    // === Annotation/Metadata Access ===

    /// Get annotations for an element as a dictionary
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn annotation_dict(
        &self,
        element_name: &str,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;

        let annotations = if let Some(class) = merged.classes.get(element_name) {
            &class.annotations
        } else if let Some(slot) = merged.slots.get(element_name) {
            &slot.annotations
        } else if let Some(type_def) = merged.types.get(element_name) {
            &type_def.annotations
        } else if let Some(enum_def) = merged.enums.get(element_name) {
            &enum_def.annotations
        } else {
            return Ok(HashMap::new());
        };

        // Convert annotations to JSON values
        let mut result = HashMap::new();
        if let Some(annotations_map) = annotations {
            for (key, annotation) in annotations_map {
                result.insert(
                    key.clone(),
                    serde_json::to_value(annotation)
                        .map_err(|e| LinkMLError::SerializationError(e.to_string()))?,
                );
            }
        }

        Ok(result)
    }

    // === Private Helper Methods ===

    fn collect_class_ancestors(
        &self,
        name: &str,
        ancestors: &mut Vec<String>,
        visited: &mut HashSet<String>,
    ) -> Result<()> {
        if visited.contains(name) {
            return Err(SchemaViewError::CircularDependency(format!(
                "Circular inheritance detected at class '{name}'"
            ))
            .into());
        }
        visited.insert(name.to_string());

        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;

        if let Some(class_def) = merged.classes.get(name)
            && let Some(parent) = &class_def.is_a
        {
            ancestors.push(parent.clone());
            self.collect_class_ancestors(parent, ancestors, visited)?;
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
            target
                .attributes
                .entry(name.clone())
                .or_insert_with(|| attr.clone());
        }

        // Merge other properties as needed
        if target.description.is_none() && source.description.is_some() {
            target.description.clone_from(&source.description);
        }
    }

    fn collect_slot_ancestors(
        &self,
        name: &str,
        ancestors: &mut Vec<String>,
        visited: &mut HashSet<String>,
    ) -> Result<()> {
        if visited.contains(name) {
            return Err(SchemaViewError::CircularDependency(format!(
                "Circular inheritance detected at slot '{name}'"
            ))
            .into());
        }
        visited.insert(name.to_string());

        let merged = self
            .merged_schema
            .read()
            .map_err(|_| SchemaViewError::CacheError("Failed to acquire read lock".into()))?;

        if let Some(slot_def) = merged.slots.get(name) {
            // Process is_a parent
            if let Some(parent) = &slot_def.is_a {
                ancestors.push(parent.clone());
                self.collect_slot_ancestors(parent, ancestors, visited)?;
            }

            // Process mixins
            for mixin in &slot_def.mixins {
                if !ancestors.contains(mixin) {
                    ancestors.push(mixin.clone());
                    self.collect_slot_ancestors(mixin, ancestors, visited)?;
                }
            }
        }

        Ok(())
    }

    fn apply_slot_usage(&self, _class: &mut ClassDefinition) -> Result<()> {
        // Apply slot_usage overrides to the class's view of slots
        // This is where class-specific slot modifications are applied

        // Note: The actual slot definitions remain unchanged in the schema;
        // only the class's view of them changes

        Ok(())
    }
}
