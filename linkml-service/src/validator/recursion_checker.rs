//! Recursion detection and validation for LinkML schemas
//!
//! This module handles validation of recursive data structures,
//! respecting RecursionOptions settings for classes.

use linkml_core::prelude::*;
use linkml_core::types::RecursionOptions;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Tracks recursion during validation
pub struct RecursionTracker {
    /// Stack of currently visited objects (by their ID)
    visited_stack: Vec<String>,

    /// Maximum depth allowed
    max_depth: usize,

    /// Current depth
    current_depth: usize,

    /// Classes that allow recursion
    recursive_classes: HashMap<String, RecursionOptions>,
}

impl RecursionTracker {
    /// Create a new recursion tracker
    pub fn new(schema: &SchemaDefinition) -> Self {
        // Collect classes with recursion options
        let mut recursive_classes = HashMap::new();

        for (class_name, class_def) in &schema.classes {
            if let Some(ref options) = class_def.recursion_options {
                recursive_classes.insert(class_name.clone(), options.clone());
            } else {
                // Auto-detect recursive classes
                if Self::is_recursive_class(class_name, class_def, schema) {
                    recursive_classes.insert(
                        class_name.clone(),
                        RecursionOptions {
                            use_box: true,
                            max_depth: Some(100), // Default max depth
                        },
                    );
                }
            }
        }

        Self {
            visited_stack: Vec::new(),
            max_depth: 100, // Global default
            current_depth: 0,
            recursive_classes,
        }
    }

    /// Check if a class is recursive (references itself directly or indirectly)
    fn is_recursive_class(
        class_name: &str,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> bool {
        // Check direct self-reference
        for slot_name in &class_def.slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                if let Some(range) = &slot.range {
                    if range == class_name {
                        return true;
                    }

                    // Check for indirect recursion through the range class
                    if let Some(range_class) = schema.classes.get(range) {
                        if Self::references_class(
                            range_class,
                            class_name,
                            schema,
                            &mut HashSet::new(),
                        ) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    /// Check if a class references another class through its slots
    fn references_class(
        class_def: &ClassDefinition,
        target: &str,
        schema: &SchemaDefinition,
        visited: &mut HashSet<String>,
    ) -> bool {
        // Prevent infinite recursion in checking
        if visited.contains(&class_def.name) {
            return false;
        }
        visited.insert(class_def.name.clone());

        for slot_name in &class_def.slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                if let Some(range) = &slot.range {
                    if range == target {
                        return true;
                    }

                    // Recursively check
                    if let Some(range_class) = schema.classes.get(range) {
                        if Self::references_class(range_class, target, schema, visited) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    /// Enter a new object during validation
    pub fn enter_object(
        &mut self,
        object_id: &str,
        class_name: &str,
    ) -> std::result::Result<(), String> {
        // Check if we're in a recursive class
        if let Some(options) = self.recursive_classes.get(class_name) {
            // Check max depth
            let max = options.max_depth.unwrap_or(self.max_depth);
            if self.current_depth >= max {
                return Err(format!(
                    "Maximum recursion depth {} exceeded for class '{}'",
                    max, class_name
                ));
            }

            // Check for circular reference
            if self.visited_stack.contains(&object_id.to_string()) {
                if !options.use_box {
                    return Err(format!(
                        "Circular reference detected for object '{}' of class '{}'. \
                        Consider setting recursion_options.use_box = true",
                        object_id, class_name
                    ));
                }
                // If use_box is true, we allow the circular reference
                // (it would be a weak reference in the actual data structure)
                return Ok(());
            }
        } else {
            // Non-recursive class shouldn't have circular references
            if self.visited_stack.contains(&object_id.to_string()) {
                return Err(format!(
                    "Unexpected circular reference in non-recursive class '{}'",
                    class_name
                ));
            }
        }

        self.visited_stack.push(object_id.to_string());
        self.current_depth += 1;
        Ok(())
    }

    /// Exit an object during validation
    pub fn exit_object(&mut self, object_id: &str) {
        if let Some(pos) = self.visited_stack.iter().position(|x| x == object_id) {
            self.visited_stack.remove(pos);
        }
        if self.current_depth > 0 {
            self.current_depth -= 1;
        }
    }

    /// Reset the tracker for a new validation
    pub fn reset(&mut self) {
        self.visited_stack.clear();
        self.current_depth = 0;
    }
}

/// Check for infinite recursion in a data instance
pub fn check_recursion(
    data: &Value,
    class_name: &str,
    schema: &SchemaDefinition,
    tracker: &mut RecursionTracker,
) -> std::result::Result<(), String> {
    // Get object ID if available
    let object_id = if let Value::Object(map) = data {
        map.get("id")
            .or_else(|| map.get("identifier"))
            .and_then(|v| v.as_str())
            .unwrap_or("anonymous")
    } else {
        "anonymous"
    };

    // Enter this object
    tracker.enter_object(object_id, class_name)?;

    // Check nested objects
    if let Value::Object(map) = data {
        if let Some(class_def) = schema.classes.get(class_name) {
            for slot_name in &class_def.slots {
                if let Some(slot_value) = map.get(slot_name) {
                    if let Some(slot) = schema.slots.get(slot_name) {
                        if let Some(range) = &slot.range {
                            // Check if this is a class reference
                            if schema.classes.contains_key(range) {
                                // Recursively check the nested object
                                if slot.multivalued.unwrap_or(false) {
                                    // Handle arrays
                                    if let Value::Array(items) = slot_value {
                                        for item in items {
                                            check_recursion(item, range, schema, tracker)?;
                                        }
                                    }
                                } else {
                                    // Handle single object
                                    check_recursion(slot_value, range, schema, tracker)?;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Exit this object
    tracker.exit_object(object_id);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direct_recursion_detection() {
        let mut schema = SchemaDefinition::default();

        // Create a self-referential class
        let mut node_class = ClassDefinition::default();
        node_class.name = "Node".to_string();
        node_class.slots = vec!["value".to_string(), "next".to_string()];
        schema.classes.insert("Node".to_string(), node_class);

        // Create slots
        let mut value_slot = SlotDefinition::default();
        value_slot.name = "value".to_string();
        value_slot.range = Some("string".to_string());
        schema.slots.insert("value".to_string(), value_slot);

        let mut next_slot = SlotDefinition::default();
        next_slot.name = "next".to_string();
        next_slot.range = Some("Node".to_string()); // Self-reference
        schema.slots.insert("next".to_string(), next_slot);

        let tracker = RecursionTracker::new(&schema);
        assert!(tracker.recursive_classes.contains_key("Node"));
    }

    #[test]
    fn test_recursion_depth_limit() {
        let mut schema = SchemaDefinition::default();

        // Create a recursive class with depth limit
        let mut tree_class = ClassDefinition::default();
        tree_class.name = "Tree".to_string();
        tree_class.slots = vec!["children".to_string()];
        tree_class.recursion_options = Some(RecursionOptions {
            use_box: true,
            max_depth: Some(3),
        });
        schema.classes.insert("Tree".to_string(), tree_class);

        let mut children_slot = SlotDefinition::default();
        children_slot.name = "children".to_string();
        children_slot.range = Some("Tree".to_string());
        children_slot.multivalued = Some(true);
        schema.slots.insert("children".to_string(), children_slot);

        let mut tracker = RecursionTracker::new(&schema);

        // Should succeed up to depth 3
        assert!(tracker.enter_object("tree1", "Tree").is_ok());
        assert!(tracker.enter_object("tree2", "Tree").is_ok());
        assert!(tracker.enter_object("tree3", "Tree").is_ok());

        // Should fail at depth 4
        assert!(tracker.enter_object("tree4", "Tree").is_err());
    }

    #[test]
    fn test_circular_reference_detection() {
        let mut schema = SchemaDefinition::default();

        // Create a class without recursion options
        let mut item_class = ClassDefinition::default();
        item_class.name = "Item".to_string();
        item_class.slots = vec!["related".to_string()];
        schema.classes.insert("Item".to_string(), item_class);

        let mut related_slot = SlotDefinition::default();
        related_slot.name = "related".to_string();
        related_slot.range = Some("Item".to_string());
        schema.slots.insert("related".to_string(), related_slot);

        let mut tracker = RecursionTracker::new(&schema);

        // First entry should succeed
        assert!(tracker.enter_object("item1", "Item").is_ok());

        // Re-entering same object should be detected as circular
        let result = tracker.enter_object("item1", "Item");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("circular reference"));
    }
}
