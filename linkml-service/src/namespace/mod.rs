//! Namespace and CURIE management for LinkML
//!
//! This module provides comprehensive namespace handling including
//! CURIE expansion/contraction, URI resolution, and namespace contexts.

pub mod curie_resolver;

pub use curie_resolver::{
    CurieResolver, NamespaceContext,
    utils::{is_absolute_uri, join_uri, local_from_uri, make_curie, split_curie},
};

use linkml_core::prelude::*;

/// Create a CURIE resolver from a schema with all imports resolved
#[must_use]
pub fn create_resolver_with_imports(
    schema: &SchemaDefinition,
    imported_schemas: &[SchemaDefinition],
) -> CurieResolver {
    let mut resolver = CurieResolver::from_schema(schema);

    // Add prefixes from imported schemas
    for imported in imported_schemas {
        for (prefix, definition) in &imported.prefixes {
            let uri = match definition {
                PrefixDefinition::Simple(uri) => uri.clone(),
                PrefixDefinition::Complex {
                    prefix_reference, ..
                } => prefix_reference.clone().unwrap_or_default(),
            };

            // Don't override existing prefixes
            if resolver.get_prefix(prefix).is_none() {
                resolver.add_prefix(prefix, &uri);
            }
        }
    }

    resolver
}

/// Resolve all CURIEs in a schema to full URIs
///
/// # Errors
///
/// Returns error if any CURIE cannot be resolved using the provided resolver.
pub fn expand_schema_curies(
    schema: &mut SchemaDefinition,
    resolver: &CurieResolver,
) -> std::result::Result<(), String> {
    // Expand class URIs
    for class in schema.classes.values_mut() {
        if let Some(uri) = &class.class_uri {
            class.class_uri = Some(resolver.expand_curie(uri).map_err(|e| e.to_string())?);
        }
    }

    // Expand slot URIs
    for slot in schema.slots.values_mut() {
        if let Some(uri) = &slot.slot_uri {
            slot.slot_uri = Some(resolver.expand_curie(uri).map_err(|e| e.to_string())?);
        }
    }

    // Expand type URIs
    for type_def in schema.types.values_mut() {
        if let Some(uri) = &type_def.uri {
            type_def.uri = Some(resolver.expand_curie(uri).map_err(|e| e.to_string())?);
        }
    }

    // Note: EnumDefinition doesn't have enum_uri field in current schema
    // If URIs are added to enums in the future, expand them here

    Ok(())
}

/// Contract all URIs in a schema to CURIEs where possible
pub fn contract_schema_uris(schema: &mut SchemaDefinition, resolver: &CurieResolver) {
    // Contract class URIs
    for class in schema.classes.values_mut() {
        if let Some(uri) = &class.class_uri {
            class.class_uri = Some(resolver.contract_uri(uri));
        }
    }

    // Contract slot URIs
    for slot in schema.slots.values_mut() {
        if let Some(uri) = &slot.slot_uri {
            slot.slot_uri = Some(resolver.contract_uri(uri));
        }
    }

    // Contract type URIs
    for type_def in schema.types.values_mut() {
        if let Some(uri) = &type_def.uri {
            type_def.uri = Some(resolver.contract_uri(uri));
        }
    }

    // Note: EnumDefinition doesn't have enum_uri field in current schema
    // If URIs are added to enums in the future, contract them here
}
