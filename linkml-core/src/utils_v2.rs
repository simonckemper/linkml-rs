//! Optimized utility functions that minimize cloning
//!
//! This module provides memory-efficient versions of utility functions that
//! use references and copy-on-write (Cow) to avoid unnecessary cloning.

use std::borrow::Cow;
use std::collections::HashSet;

use crate::error::{LinkMLError, Result};
use crate::types::{
    SchemaDefinition, ClassDefinition, SlotDefinition
};

/// Check if a given name is a built-in type (no clone)
pub fn is_builtin_type(name: &str) -> bool {
    matches!(
        name,
        "string" | "integer" | "boolean" | "float" | "double" | "decimal" | 
        "time" | "date" | "datetime" | "date_or_datetime" | "uriorcurie" | 
        "curie" | "uri" | "ncname" | "objectidentifier" | "nodeidentifier" |
        "jsonpointer" | "jsonpath" | "sparqlpath"
    )
}

/// Get all slot names for a class including inherited slots (returns references)
pub fn get_class_slots<'a>(
    class: &'a ClassDefinition,
    schema: &'a SchemaDefinition,
) -> Result<Vec<&'a str>> {
    let mut slots = Vec::new();
    let mut seen = HashSet::new();
    
    // Add direct slots
    for slot_name in &class.slots {
        if seen.insert(slot_name.as_str()) {
            slots.push(slot_name.as_str());
        }
    }
    
    // Add attribute names
    for attr_name in class.attributes.keys() {
        if seen.insert(attr_name.as_str()) {
            slots.push(attr_name.as_str());
        }
    }
    
    // Add slots from mixins
    for mixin_name in &class.mixins {
        if let Some(mixin_class) = schema.classes.get(mixin_name) {
            let mixin_slots = get_class_slots(mixin_class, schema)?;
            for slot in mixin_slots {
                if seen.insert(slot) {
                    slots.push(slot);
                }
            }
        }
    }
    
    // Add slots from parent class
    if let Some(parent_name) = &class.is_a {
        if let Some(parent_class) = schema.classes.get(parent_name) {
            let parent_slots = get_class_slots(parent_class, schema)?;
            for slot in parent_slots {
                if seen.insert(slot) {
                    slots.push(slot);
                }
            }
        }
    }
    
    Ok(slots)
}

/// Merge slot definitions efficiently using Cow
pub fn merge_slot_definitions_cow<'a>(
    base: &'a SlotDefinition,
    override_def: &'a SlotDefinition,
) -> Cow<'a, SlotDefinition> {
    // Check if any fields differ
    let needs_merge = override_def.description.is_some() ||
        override_def.range.is_some() ||
        override_def.required.is_some() ||
        override_def.multivalued.is_some() ||
        override_def.pattern.is_some() ||
        !override_def.aliases.is_empty() ||
        !override_def.mixins.is_empty();
    
    if !needs_merge {
        // No changes needed, return base as borrowed
        return Cow::Borrowed(base);
    }
    
    // Create merged definition only when needed
    Cow::Owned(SlotDefinition {
        name: override_def.name.clone(),
        description: override_def.description.as_ref()
            .or(base.description.as_ref())
            .cloned(),
        range: override_def.range.as_ref()
            .or(base.range.as_ref())
            .cloned(),
        required: override_def.required.or(base.required),
        multivalued: override_def.multivalued.or(base.multivalued),
        identifier: override_def.identifier.or(base.identifier),
        pattern: override_def.pattern.as_ref()
            .or(base.pattern.as_ref())
            .cloned(),
        minimum_value: override_def.minimum_value.as_ref()
            .or(base.minimum_value.as_ref())
            .cloned(),
        maximum_value: override_def.maximum_value.as_ref()
            .or(base.maximum_value.as_ref())
            .cloned(),
        permissible_values: if override_def.permissible_values.is_empty() {
            base.permissible_values.clone()
        } else {
            override_def.permissible_values.clone()
        },
        slot_uri: override_def.slot_uri.as_ref()
            .or(base.slot_uri.as_ref())
            .cloned(),
        aliases: merge_vec_cow(&base.aliases, &override_def.aliases),
        is_a: override_def.is_a.as_ref()
            .or(base.is_a.as_ref())
            .cloned(),
        mixins: if override_def.mixins.is_empty() {
            base.mixins.clone()
        } else {
            override_def.mixins.clone()
        },
        inverse: override_def.inverse.as_ref()
            .or(base.inverse.as_ref())
            .cloned(),
        default: override_def.default.as_ref()
            .or(base.default.as_ref())
            .cloned(),
        inlined: override_def.inlined.or(base.inlined),
        inlined_as_list: override_def.inlined_as_list.or(base.inlined_as_list),
        any_of: override_def.any_of.as_ref()
            .or(base.any_of.as_ref())
            .cloned(),
        all_of: override_def.all_of.as_ref()
            .or(base.all_of.as_ref())
            .cloned(),
        exactly_one_of: override_def.exactly_one_of.as_ref()
            .or(base.exactly_one_of.as_ref())
            .cloned(),
        none_of: override_def.none_of.as_ref()
            .or(base.none_of.as_ref())
            .cloned(),
        equals_expression: override_def.equals_expression.as_ref()
            .or(base.equals_expression.as_ref())
            .cloned(),
        rules: override_def.rules.as_ref()
            .or(base.rules.as_ref())
            .cloned(),
        equals_string_in: override_def.equals_string_in.as_ref()
            .or(base.equals_string_in.as_ref())
            .cloned(),
        structured_pattern: override_def.structured_pattern.as_ref()
            .or(base.structured_pattern.as_ref())
            .cloned(),
        annotations: crate::annotations::merge_annotations(
            base.annotations.as_ref(),
            override_def.annotations.as_ref(),
        ),
        see_also: merge_vec_cow(&base.see_also, &override_def.see_also),
        examples: merge_vec_cow(&base.examples, &override_def.examples),
        deprecated: override_def.deprecated.as_ref()
            .or(base.deprecated.as_ref())
            .cloned(),
        todos: merge_vec_cow(&base.todos, &override_def.todos),
        notes: merge_vec_cow(&base.notes, &override_def.notes),
        comments: merge_vec_cow(&base.comments, &override_def.comments),
        ..base.clone() // Only clone remaining fields
    })
}

/// Merge two vectors efficiently
fn merge_vec_cow<T: Clone + PartialEq + Eq + std::hash::Hash>(base: &[T], override_vec: &[T]) -> Vec<T> {
    if override_vec.is_empty() {
        return base.to_vec();
    }
    
    let mut result = base.to_vec();
    let base_set: HashSet<_> = base.iter().collect();
    
    for item in override_vec {
        if !base_set.contains(item) {
            result.push(item.clone());
        }
    }
    
    result
}

/// Get effective slot definition for a class (returns reference when possible)
pub fn get_slot_definition<'a>(
    schema: &'a SchemaDefinition,
    class: &'a ClassDefinition,
    slot_name: &str,
) -> Result<Cow<'a, SlotDefinition>> {
    // Check slot usage first
    if let Some(usage) = class.slot_usage.get(slot_name) {
        // Check if we have a base definition to merge with
        if let Some(base) = schema.slots.get(slot_name) {
            return Ok(merge_slot_definitions_cow(base, usage));
        } else {
            return Ok(Cow::Borrowed(usage));
        }
    }
    
    // Check attributes
    if let Some(attr) = class.attributes.get(slot_name) {
        return Ok(Cow::Borrowed(attr));
    }
    
    // Check schema-level slots
    if let Some(slot) = schema.slots.get(slot_name) {
        return Ok(Cow::Borrowed(slot));
    }
    
    // Check inherited slots
    if let Some(parent_name) = &class.is_a {
        if let Some(parent_class) = schema.classes.get(parent_name) {
            return get_slot_definition(schema, parent_class, slot_name);
        }
    }
    
    // Check mixin slots
    for mixin_name in &class.mixins {
        if let Some(mixin_class) = schema.classes.get(mixin_name) {
            if let Ok(slot) = get_slot_definition(schema, mixin_class, slot_name) {
                return Ok(slot);
            }
        }
    }
    
    Err(LinkMLError::Other { 
        message: format!("Slot '{}' not found", slot_name),
        source: None
    })
}

/// Check if a type is valid (no clone needed)
pub fn is_valid_type<'a>(
    schema: &'a SchemaDefinition,
    type_name: &str,
) -> bool {
    is_builtin_type(type_name) ||
    schema.types.contains_key(type_name) ||
    schema.classes.contains_key(type_name) ||
    schema.enums.contains_key(type_name)
}

/// Get all imports recursively (returns references)
pub fn get_all_imports<'a>(
    schema: &'a SchemaDefinition,
    visited: &mut HashSet<&'a str>,
) -> Vec<&'a str> {
    let mut all_imports = Vec::new();
    
    for import in &schema.imports {
        if visited.insert(import.as_str()) {
            all_imports.push(import.as_str());
        }
    }
    
    all_imports
}

/// Get class hierarchy (returns references)
pub fn get_class_hierarchy<'a>(
    schema: &'a SchemaDefinition,
    class_name: &'a str,
) -> Result<Vec<&'a str>> {
    let mut hierarchy = vec![class_name];
    let mut current = class_name;
    let mut seen = HashSet::new();
    
    while let Some(class) = schema.classes.get(current) {
        if let Some(parent) = &class.is_a {
            if !seen.insert(parent.as_str()) {
                return Err(LinkMLError::SchemaValidationError {
                    message: format!("Circular inheritance detected at class '{}'", parent),
                    element: Some(parent.to_string()),
                });
            }
            hierarchy.push(parent.as_str());
            current = parent.as_str();
        } else {
            break;
        }
    }
    
    Ok(hierarchy)
}

/// Convert camelCase to snake_case efficiently
pub fn camel_to_snake(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 5);
    let mut prev_upper = false;
    
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 && !prev_upper {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap_or(ch));
        prev_upper = ch.is_uppercase();
    }
    
    result
}

/// Convert snake_case to camelCase efficiently
pub fn snake_to_camel(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut capitalize_next = false;
    
    for ch in s.chars() {
        if ch == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(ch.to_uppercase().next().unwrap_or(ch));
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }
    
    result
}

/// Get all classes that inherit from a given class (returns references)
pub fn get_subclasses<'a>(
    schema: &'a SchemaDefinition,
    parent_name: &str,
) -> Vec<&'a str> {
    schema.classes
        .iter()
        .filter(|(_, class)| {
            class.is_a.as_deref() == Some(parent_name) ||
            class.mixins.iter().any(|m| m == parent_name)
        })
        .map(|(name, _)| name.as_str())
        .collect()
}

/// Check if a class is abstract
pub fn is_abstract_class(class: &ClassDefinition) -> bool {
    class.abstract_.unwrap_or(false) || class.mixin.unwrap_or(false)
}

/// Get URI for a given element efficiently
pub fn get_element_uri<'a>(
    element_name: &'a str,
    uri_field: Option<&'a str>,
    schema: &'a SchemaDefinition,
) -> Cow<'a, str> {
    if let Some(uri) = uri_field {
        return Cow::Borrowed(uri);
    }
    
    // Generate URI from schema base
    if let Some(base) = schema.default_prefix.as_ref() {
        Cow::Owned(format!("{}:{}", base, element_name))
    } else {
        Cow::Borrowed(element_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_builtin_type_no_clone() {
        // This function now takes &str, no cloning needed
        assert!(is_builtin_type("string"));
        assert!(is_builtin_type("integer"));
        assert!(!is_builtin_type("MyCustomType"));
    }

    #[test]
    fn test_camel_snake_conversion() {
        assert_eq!(camel_to_snake("camelCase"), "camel_case");
        assert_eq!(camel_to_snake("HTTPServer"), "httpserver");
        assert_eq!(snake_to_camel("snake_case"), "snakeCase");
        assert_eq!(snake_to_camel("http_server"), "httpServer");
    }
}