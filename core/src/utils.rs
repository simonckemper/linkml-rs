//! Utility functions and helpers for LinkML operations

use crate::error::{LinkMLError, Result};
use crate::types::{SchemaDefinition, SlotDefinition};
use indexmap::IndexMap;
use std::collections::{HashSet, VecDeque};
use std::hash::BuildHasher;

/// Check if a string is a valid `LinkML` identifier
#[must_use]
pub fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    // Must start with letter or underscore
    let Some(first) = s.chars().next() else {
        return false; // Empty string case already handled above
    };
    if !first.is_alphabetic() && first != '_' {
        return false;
    }

    // Rest must be alphanumeric or underscore
    s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// Normalize a URI by removing trailing slashes and fragments
#[must_use]
pub fn normalize_uri(uri: &str) -> String {
    let mut normalized = uri.trim().to_string();

    // Remove fragment
    if let Some(pos) = normalized.find('#') {
        normalized.truncate(pos);
    }

    // Remove trailing slash
    if normalized.ends_with('/') && normalized.len() > 1 {
        normalized.pop();
    }

    normalized
}

/// Extract prefix from a CURIE (Compact URI)
#[must_use]
pub fn extract_prefix(curie: &str) -> Option<(&str, &str)> {
    curie.split_once(':')
}

/// Expand a CURIE using prefix map
///
/// # Errors
///
/// Returns an error if the prefix is not found in the prefix map.
pub fn expand_curie(curie: &str, prefixes: &IndexMap<String, String>) -> Result<String> {
    if let Some((prefix, local)) = extract_prefix(curie) {
        if let Some(expansion) = prefixes.get(prefix) {
            Ok(format!("{expansion}{local}"))
        } else {
            Err(LinkMLError::other(format!("Unknown prefix: {prefix}")))
        }
    } else {
        // Not a CURIE, return as-is
        Ok(curie.to_string())
    }
}

/// Get all slots for a class including inherited ones
///
/// # Errors
///
/// Returns an error if the class is not found in the schema.
pub fn get_class_slots<S>(
    schema: &SchemaDefinition,
    class_name: &str,
    visited: &mut HashSet<String, S>,
) -> Result<Vec<String>>
where
    S: BuildHasher,
{
    if visited.contains(class_name) {
        return Ok(Vec::new());
    }
    visited.insert(class_name.to_string());

    let class = schema
        .classes
        .get(class_name)
        .ok_or_else(|| LinkMLError::other(format!("Class not found: {class_name}")))?;

    let mut slots = Vec::new();

    // Add direct slots
    slots.extend(class.slots.clone());

    // Add attribute names
    slots.extend(class.attributes.keys().cloned());

    // Add inherited slots from parent
    if let Some(parent) = &class.is_a {
        let parent_slots = get_class_slots(schema, parent, visited)?;
        for slot in parent_slots {
            if !slots.contains(&slot) {
                slots.push(slot);
            }
        }
    }

    // Add slots from mixins
    for mixin in &class.mixins {
        let mixin_slots = get_class_slots(schema, mixin, visited)?;
        for slot in mixin_slots {
            if !slots.contains(&slot) {
                slots.push(slot);
            }
        }
    }

    Ok(slots)
}

/// Check if a class is a subclass of another
///
/// # Errors
///
/// Returns an error if there are issues accessing the schema structure.
pub fn is_subclass_of(schema: &SchemaDefinition, child: &str, parent: &str) -> Result<bool> {
    if child == parent {
        return Ok(true);
    }

    let mut queue = VecDeque::new();
    queue.push_back(child);

    let mut visited = HashSet::new();

    while let Some(current) = queue.pop_front() {
        if visited.contains(current) {
            continue;
        }
        visited.insert(current);

        if let Some(class) = schema.classes.get(current) {
            // Check direct parent
            if let Some(is_a) = &class.is_a {
                if is_a == parent {
                    return Ok(true);
                }
                queue.push_back(is_a);
            }

            // Check mixins
            for mixin in &class.mixins {
                if mixin == parent {
                    return Ok(true);
                }
                queue.push_back(mixin);
            }
        }
    }

    Ok(false)
}

/// Merge slot definitions, with override taking precedence
#[must_use]
pub fn merge_slot_definitions(
    base: &SlotDefinition,
    override_def: &SlotDefinition,
) -> SlotDefinition {
    SlotDefinition {
        name: override_def.name.clone(),
        description: override_def
            .description
            .clone()
            .or_else(|| base.description.clone()),
        range: override_def.range.clone().or_else(|| base.range.clone()),
        domain: override_def.domain.clone().or_else(|| base.domain.clone()),
        required: override_def.required.or(base.required),
        multivalued: override_def.multivalued.or(base.multivalued),
        identifier: override_def.identifier.or(base.identifier),
        pattern: override_def
            .pattern
            .clone()
            .or_else(|| base.pattern.clone()),
        minimum_value: override_def
            .minimum_value
            .clone()
            .or_else(|| base.minimum_value.clone()),
        maximum_value: override_def
            .maximum_value
            .clone()
            .or_else(|| base.maximum_value.clone()),
        permissible_values: if override_def.permissible_values.is_empty() {
            base.permissible_values.clone()
        } else {
            override_def.permissible_values.clone()
        },
        min_length: override_def.min_length.or(base.min_length),
        max_length: override_def.max_length.or(base.max_length),
        key: override_def.key.or(base.key),
        readonly: override_def.readonly.or(base.readonly),
        slot_uri: override_def
            .slot_uri
            .clone()
            .or_else(|| base.slot_uri.clone()),
        aliases: merge_vec(&base.aliases, &override_def.aliases),
        is_a: override_def.is_a.clone().or_else(|| base.is_a.clone()),
        mixins: override_def.mixins.clone(),
        inverse: override_def
            .inverse
            .clone()
            .or_else(|| base.inverse.clone()),
        default: override_def
            .default
            .clone()
            .or_else(|| base.default.clone()),
        inlined: override_def.inlined.or(base.inlined),
        ifabsent: override_def
            .ifabsent
            .clone()
            .or_else(|| base.ifabsent.clone()),
        inlined_as_list: override_def.inlined_as_list.or(base.inlined_as_list),
        any_of: override_def.any_of.clone().or_else(|| base.any_of.clone()),
        all_of: override_def.all_of.clone().or_else(|| base.all_of.clone()),
        exactly_one_of: override_def
            .exactly_one_of
            .clone()
            .or_else(|| base.exactly_one_of.clone()),
        none_of: override_def
            .none_of
            .clone()
            .or_else(|| base.none_of.clone()),
        equals_expression: override_def
            .equals_expression
            .clone()
            .or_else(|| base.equals_expression.clone()),
        rules: override_def.rules.clone().or_else(|| base.rules.clone()),
        // The equals string in
        equals_string_in: override_def
            .equals_string_in
            .clone()
            .or_else(|| base.equals_string_in.clone()),
        structured_pattern: override_def
            .structured_pattern
            .clone()
            .or_else(|| base.structured_pattern.clone()),
        annotations: crate::annotations::merge_annotations(
            base.annotations.as_ref(),
            override_def.annotations.as_ref(),
        ),
        see_also: merge_vec(&base.see_also, &override_def.see_also),
        examples: merge_vec(&base.examples, &override_def.examples),
        deprecated: override_def
            .deprecated
            .clone()
            .or_else(|| base.deprecated.clone()),
        todos: merge_vec(&base.todos, &override_def.todos),
        notes: merge_vec(&base.notes, &override_def.notes),
        comments: merge_vec(&base.comments, &override_def.comments),
        rank: override_def.rank.or(base.rank),
        unique: override_def.unique.or(base.unique),
        ordered: override_def.ordered.or(base.ordered),
        unique_keys: merge_vec(&base.unique_keys, &override_def.unique_keys),
        exact_mappings: merge_vec(&base.exact_mappings, &override_def.exact_mappings),
        close_mappings: merge_vec(&base.close_mappings, &override_def.close_mappings),
        related_mappings: merge_vec(&base.related_mappings, &override_def.related_mappings),
        narrow_mappings: merge_vec(&base.narrow_mappings, &override_def.narrow_mappings),
        broad_mappings: merge_vec(&base.broad_mappings, &override_def.broad_mappings),
    }
}

/// Helper to merge two vectors, removing duplicates
fn merge_vec<T: Clone + PartialEq>(base: &[T], override_vec: &[T]) -> Vec<T> {
    let mut result = base.to_vec();
    for item in override_vec {
        if !result.contains(item) {
            result.push(item.clone());
        }
    }
    result
}

/// Get the effective slot definition for a class
///
/// # Errors
///
/// Returns an error if the class or slot is not found.
pub fn get_effective_slot(
    schema: &SchemaDefinition,
    class_name: &str,
    slot_name: &str,
) -> Result<SlotDefinition> {
    let class = schema
        .classes
        .get(class_name)
        .ok_or_else(|| LinkMLError::other(format!("Class not found: {class_name}")))?;

    // Check slot usage first
    if let Some(usage) = class.slot_usage.get(slot_name) {
        if let Some(base) = schema.slots.get(slot_name) {
            return Ok(merge_slot_definitions(base, usage));
        }
        return Ok(usage.clone());
    }

    // Check attributes
    if let Some(attr) = class.attributes.get(slot_name) {
        return Ok(attr.clone());
    }

    // Check schema-level slots
    if let Some(slot) = schema.slots.get(slot_name) {
        return Ok(slot.clone());
    }

    Err(LinkMLError::other(format!("Slot not found: {slot_name}")))
}

/// Topologically sort classes based on inheritance
///
/// # Errors
///
/// Returns an error if circular inheritance is detected.
pub fn topological_sort_classes(schema: &SchemaDefinition) -> Result<Vec<String>> {
    let mut sorted = Vec::new();
    let mut visited = HashSet::new();
    let mut visiting = HashSet::new();

    for class_name in schema.classes.keys() {
        visit(class_name, schema, &mut sorted, &mut visited, &mut visiting)?;
    }

    Ok(sorted)
}

/// Helper function for topological sort
fn visit(
    name: &str,
    schema: &SchemaDefinition,
    sorted: &mut Vec<String>,
    visited: &mut HashSet<String>,
    visiting: &mut HashSet<String>,
) -> Result<()> {
    if visited.contains(name) {
        return Ok(());
    }

    if visiting.contains(name) {
        return Err(LinkMLError::other(format!(
            "Circular inheritance detected at: {name}"
        )));
    }

    visiting.insert(name.to_string());

    if let Some(class) = schema.classes.get(name) {
        // Visit parent first
        if let Some(parent) = &class.is_a {
            visit(parent, schema, sorted, visited, visiting)?;
        }

        // Visit mixins
        for mixin in &class.mixins {
            visit(mixin, schema, sorted, visited, visiting)?;
        }
    }

    visiting.remove(name);
    visited.insert(name.to_string());
    sorted.push(name.to_string());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_identifier() {
        assert!(is_valid_identifier("valid_name"));
        assert!(is_valid_identifier("_private"));
        assert!(is_valid_identifier("Class123"));

        assert!(!is_valid_identifier(""));
        assert!(!is_valid_identifier("123invalid"));
        assert!(!is_valid_identifier("invalid-name"));
        assert!(!is_valid_identifier("invalid name"));
    }

    #[test]
    fn test_normalize_uri() {
        assert_eq!(normalize_uri("http://example.org/"), "http://example.org");
        assert_eq!(
            normalize_uri("http://example.org/path#fragment"),
            "http://example.org/path"
        );
        assert_eq!(normalize_uri(" http://example.org "), "http://example.org");
    }

    #[test]
    fn test_extract_prefix() {
        assert_eq!(extract_prefix("ex:Person"), Some(("ex", "Person")));
        assert_eq!(extract_prefix("no_prefix"), None);
    }

    #[test]
    fn test_expand_curie() {
        let mut prefixes = IndexMap::new();
        prefixes.insert("ex".to_string(), "http://example.org/".to_string());

        let result = expand_curie("ex:Person", &prefixes);
        assert!(result.is_ok());
        assert_eq!(result.ok(), Some("http://example.org/Person".to_string()));

        assert!(expand_curie("unknown:Person", &prefixes).is_err());
    }
}
