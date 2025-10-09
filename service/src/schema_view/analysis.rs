//! Schema analysis utilities for computing statistics and usage patterns

use std::collections::{HashMap, HashSet};

use super::view::{SchemaView, SchemaViewError};
use linkml_core::error::Result;

/// Statistics about a `LinkML` schema
#[derive(Debug, Clone, Default)]
pub struct SchemaStatistics {
    /// Number of classes
    pub class_count: usize,

    /// Number of slots
    pub slot_count: usize,

    /// Number of types
    pub type_count: usize,

    /// Number of enums
    pub enum_count: usize,

    /// Number of subsets
    pub subset_count: usize,

    /// Average slots per class
    pub avg_slots_per_class: f64,

    /// Maximum inheritance depth
    pub max_inheritance_depth: usize,

    /// Number of root classes
    pub root_class_count: usize,

    /// Number of leaf classes
    pub leaf_class_count: usize,

    /// Number of mixin classes
    pub mixin_count: usize,

    /// Number of abstract classes
    pub abstract_class_count: usize,

    /// Total unique imports
    pub import_count: usize,
}

/// Information about where an element is used
#[derive(Debug, Clone, Default)]
pub struct UsageInfo {
    /// Classes that reference this element
    pub used_by_classes: Vec<String>,

    /// Slots that reference this element (e.g., as range)
    pub used_by_slots: Vec<String>,

    /// Whether this element is used as a mixin
    pub used_as_mixin: bool,

    /// Whether this element is used as a type range
    pub used_as_range: bool,

    /// Whether this element is used in `slot_usage`
    pub used_in_slot_usage: bool,

    /// Total usage count
    pub total_usage_count: usize,
}

/// Index of element usage throughout the schema
#[derive(Debug, Clone)]
pub struct UsageIndex {
    /// Usage information for each element
    usage_map: HashMap<String, UsageInfo>,
}

impl UsageIndex {
    /// Build a usage index for the schema
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn build(schema_view: &SchemaView) -> linkml_core::error::Result<Self> {
        let mut index = Self {
            usage_map: HashMap::new(),
        };

        // Analyze class usage
        for (class_name, class_def) in schema_view.all_classes()? {
            // Track parent class usage
            if let Some(parent) = &class_def.is_a {
                index.record_class_usage(parent, &class_name);
            }

            // Track mixin usage
            for mixin in &class_def.mixins {
                index.record_mixin_usage(mixin, &class_name);
            }

            // Track slot usage
            for slot_name in &class_def.slots {
                index.record_slot_usage_by_class(slot_name, &class_name);
            }

            // Track slot_usage overrides
            for slot_name in class_def.slot_usage.keys() {
                index.record_slot_usage_override(slot_name, &class_name);
            }
        }

        // Analyze slot range usage
        for (slot_name, slot_def) in schema_view.all_slots()? {
            if let Some(range) = &slot_def.range {
                index.record_range_usage(range, &slot_name);
            }
        }

        // Calculate total usage counts
        for usage in index.usage_map.values_mut() {
            usage.total_usage_count = usage.used_by_classes.len() + usage.used_by_slots.len();
        }

        Ok(index)
    }

    /// Get usage information for an element
    #[must_use]
    pub fn get_usage(&self, element_name: &str) -> Option<&UsageInfo> {
        self.usage_map.get(element_name)
    }

    /// Find unused elements
    #[must_use]
    pub fn find_unused_elements(&self) -> Vec<String> {
        self.usage_map
            .iter()
            .filter(|(_, usage)| usage.total_usage_count == 0)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Find heavily used elements
    #[must_use]
    pub fn find_heavily_used_elements(&self, threshold: usize) -> Vec<(String, usize)> {
        self.usage_map
            .iter()
            .filter(|(_, usage)| usage.total_usage_count >= threshold)
            .map(|(name, usage)| (name.clone(), usage.total_usage_count))
            .collect()
    }

    fn record_class_usage(&mut self, parent: &str, child: &str) {
        let usage = self.usage_map.entry(parent.to_string()).or_default();
        usage.used_by_classes.push(child.to_string());
    }

    fn record_mixin_usage(&mut self, mixin: &str, class: &str) {
        let usage = self.usage_map.entry(mixin.to_string()).or_default();
        usage.used_by_classes.push(class.to_string());
        usage.used_as_mixin = true;
    }

    fn record_slot_usage_by_class(&mut self, slot: &str, class: &str) {
        let usage = self.usage_map.entry(slot.to_string()).or_default();
        usage.used_by_classes.push(class.to_string());
    }

    fn record_slot_usage_override(&mut self, slot: &str, class: &str) {
        let usage = self.usage_map.entry(slot.to_string()).or_default();
        usage.used_in_slot_usage = true;
        if !usage.used_by_classes.contains(&class.to_string()) {
            usage.used_by_classes.push(class.to_string());
        }
    }

    fn record_range_usage(&mut self, range: &str, slot: &str) {
        let usage = self.usage_map.entry(range.to_string()).or_default();
        usage.used_by_slots.push(slot.to_string());
        usage.used_as_range = true;
    }
}

/// Analyzer for computing schema statistics and patterns
pub struct SchemaAnalyzer<'a> {
    schema_view: &'a SchemaView,
}

impl<'a> SchemaAnalyzer<'a> {
    /// Create a new schema analyzer
    #[must_use]
    pub fn new(schema_view: &'a SchemaView) -> Self {
        Self { schema_view }
    }

    /// Compute comprehensive statistics about the schema
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn compute_statistics(&self) -> linkml_core::error::Result<SchemaStatistics> {
        // Basic counts
        let class_count = self.schema_view.all_classes()?.len();
        let slot_count = self.schema_view.all_slots()?.len();
        let type_count = self.schema_view.all_types()?.len();
        let enum_count = self.schema_view.all_enums()?.len();

        let mut stats = SchemaStatistics {
            class_count,
            slot_count,
            type_count,
            enum_count,
            ..Default::default()
        };

        // Class analysis
        let all_classes = self.schema_view.all_classes()?;
        let mut total_slots = 0;
        let mut max_depth = 0;

        for (class_name, class_def) in &all_classes {
            // Count slots
            let slot_count = self.schema_view.class_slots(class_name)?.len();
            total_slots += slot_count;

            // Check if abstract
            if class_def.abstract_.unwrap_or(false) {
                stats.abstract_class_count += 1;
            }

            // Check if mixin
            if class_def.mixin.unwrap_or(false) {
                stats.mixin_count += 1;
            }

            // Calculate inheritance depth
            let ancestors = self.schema_view.class_ancestors(class_name)?;
            max_depth = max_depth.max(ancestors.len());
        }

        // Calculate averages
        if stats.class_count > 0 {
            stats.avg_slots_per_class = total_slots as f64 / stats.class_count as f64;
        }
        stats.max_inheritance_depth = max_depth;

        // Count root and leaf classes
        use super::navigation::ClassNavigator;
        let navigator = ClassNavigator::new(self.schema_view);
        stats.root_class_count = navigator.get_root_classes()?.len();
        stats.leaf_class_count = navigator.get_leaf_classes()?.len();

        Ok(stats)
    }

    /// Find potential issues in the schema
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn find_potential_issues(&self) -> linkml_core::error::Result<Vec<String>> {
        let mut issues = Vec::new();

        // Check for unused elements
        let usage_index = self.schema_view.usage_index()?;
        let unused = usage_index.find_unused_elements();
        for element in unused {
            issues.push(format!("Unused element: {element}"));
        }

        // Check for very deep inheritance
        let stats = self.compute_statistics()?;
        if stats.max_inheritance_depth > 5 {
            issues.push(format!(
                "Very deep inheritance hierarchy detected (depth: {})",
                stats.max_inheritance_depth
            ));
        }

        // Check for classes with too many slots
        for (class_name, _) in self.schema_view.all_classes()? {
            let slot_count = self.schema_view.class_slots(&class_name)?.len();
            if slot_count > 50 {
                issues.push(format!(
                    "Class '{class_name}' has {slot_count} slots, consider breaking it down"
                ));
            }
        }

        // Check for circular dependencies
        // Note: This is a simplified check; a full implementation would need graph analysis
        for (class_name, class_def) in self.schema_view.all_classes()? {
            if let Some(parent) = &class_def.is_a
                && parent == &class_name
            {
                issues.push(format!("Class '{class_name}' inherits from itself"));
            }
        }

        Ok(issues)
    }

    /// Find all elements matching a pattern
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns `SchemaViewError::LoadError` if the regex pattern is invalid
    /// Returns schema view errors if element enumeration fails
    pub fn find_elements_by_pattern(&self, pattern: &str) -> Result<HashMap<String, Vec<String>>> {
        let mut results = HashMap::new();
        let regex = regex::Regex::new(pattern)
            .map_err(|e| SchemaViewError::LoadError(format!("Invalid regex pattern: {e}")))?;

        // Search classes
        let mut matching_classes = Vec::new();
        for class_name in self.schema_view.all_class_names()? {
            if regex.is_match(&class_name) {
                matching_classes.push(class_name);
            }
        }
        if !matching_classes.is_empty() {
            results.insert("classes".to_string(), matching_classes);
        }

        // Search slots
        let mut matching_slots = Vec::new();
        for (slot_name, _) in self.schema_view.all_slots()? {
            if regex.is_match(&slot_name) {
                matching_slots.push(slot_name);
            }
        }
        if !matching_slots.is_empty() {
            results.insert("slots".to_string(), matching_slots);
        }

        // Search enums
        let mut matching_enums = Vec::new();
        for (enum_name, _) in self.schema_view.all_enums()? {
            if regex.is_match(&enum_name) {
                matching_enums.push(enum_name);
            }
        }
        if !matching_enums.is_empty() {
            results.insert("enums".to_string(), matching_enums);
        }

        Ok(results)
    }

    /// Generate a dependency graph for classes
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns schema view errors if class enumeration or slot resolution fails
    pub fn generate_class_dependency_graph(&self) -> Result<HashMap<String, HashSet<String>>> {
        let mut graph = HashMap::new();

        for (class_name, class_def) in self.schema_view.all_classes()? {
            let mut dependencies = HashSet::new();

            // Add parent class
            if let Some(parent) = &class_def.is_a {
                dependencies.insert(parent.clone());
            }

            // Add mixins
            for mixin in &class_def.mixins {
                dependencies.insert(mixin.clone());
            }

            // Add slot ranges that are classes
            for slot_name in &class_def.slots {
                if let Some(slot) = self.schema_view.get_slot(slot_name)?
                    && let Some(range) = &slot.range
                {
                    // Check if range is a class
                    if self.schema_view.get_class(range)?.is_some() {
                        dependencies.insert(range.clone());
                    }
                }
            }

            graph.insert(class_name, dependencies);
        }

        Ok(graph)
    }
}
