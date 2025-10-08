//! Semantic equivalence checker for LinkML schemas and data
//!
//! This module provides deep semantic comparison rather than syntactic matching.
//! It handles order-independent collections, whitespace normalization, and provides
//! detailed diff reporting for debugging failed round-trips.

use indexmap::IndexMap;
use linkml_core::prelude::*;
use std::collections::BTreeSet;

/// Result of semantic equivalence check
#[derive(Debug, Clone, PartialEq)]
pub struct EquivalenceResult {
    /// Whether the schemas/data are equivalent
    pub is_equivalent: bool,
    /// Detailed differences if not equivalent
    pub differences: Vec<Difference>,
}

/// Type of difference found during comparison
#[derive(Debug, Clone, PartialEq)]
pub enum Difference {
    /// Missing element in reconstructed schema
    MissingElement {
        path: String,
        element_type: String,
        name: String,
    },
    /// Extra element in reconstructed schema
    ExtraElement {
        path: String,
        element_type: String,
        name: String,
    },
    /// Type mismatch
    TypeMismatch {
        path: String,
        expected: String,
        actual: String,
    },
    /// Value mismatch
    ValueMismatch {
        path: String,
        field: String,
        expected: String,
        actual: String,
    },
    /// Constraint violation
    ConstraintViolation {
        path: String,
        constraint_type: String,
        expected: String,
        actual: String,
    },
    /// Metadata difference (descriptions, annotations)
    MetadataDifference {
        path: String,
        field: String,
        expected: Option<String>,
        actual: Option<String>,
    },
}

impl std::fmt::Display for Difference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingElement {
                path,
                element_type,
                name,
            } => write!(f, "Missing {element_type} '{name}' at {path}"),
            Self::ExtraElement {
                path,
                element_type,
                name,
            } => write!(f, "Extra {element_type} '{name}' at {path}"),
            Self::TypeMismatch {
                path,
                expected,
                actual,
            } => write!(
                f,
                "Type mismatch at {path}: expected {expected}, got {actual}"
            ),
            Self::ValueMismatch {
                path,
                field,
                expected,
                actual,
            } => write!(
                f,
                "Value mismatch at {path}.{field}: expected '{expected}', got '{actual}'"
            ),
            Self::ConstraintViolation {
                path,
                constraint_type,
                expected,
                actual,
            } => write!(
                f,
                "Constraint violation at {path}: {constraint_type} expected '{expected}', got '{actual}'"
            ),
            Self::MetadataDifference {
                path,
                field,
                expected,
                actual,
            } => {
                let exp_str = expected.as_deref().unwrap_or("<none>");
                let act_str = actual.as_deref().unwrap_or("<none>");
                write!(
                    f,
                    "Metadata difference at {path}.{field}: expected '{exp_str}', got '{act_str}'"
                )
            }
        }
    }
}

impl EquivalenceResult {
    /// Create a result indicating equivalence
    #[must_use]
    pub fn equivalent() -> Self {
        Self {
            is_equivalent: true,
            differences: Vec::new(),
        }
    }

    /// Create a result indicating differences
    #[must_use]
    pub fn different(differences: Vec<Difference>) -> Self {
        Self {
            is_equivalent: false,
            differences,
        }
    }

    /// Get formatted report of differences
    #[must_use]
    pub fn report(&self) -> String {
        if self.is_equivalent {
            return "Schemas are semantically equivalent".to_string();
        }

        let mut report = format!("Found {} differences:\n\n", self.differences.len());
        for (i, diff) in self.differences.iter().enumerate() {
            report.push_str(&format!("{}. {diff}\n", i + 1));
        }
        report
    }
}

/// Compare two schemas for semantic equivalence
#[must_use]
pub fn compare_schemas(
    original: &SchemaDefinition,
    reconstructed: &SchemaDefinition,
) -> EquivalenceResult {
    let mut differences = Vec::new();

    // Compare schema metadata
    compare_schema_metadata(original, reconstructed, &mut differences);

    // Compare classes (order-independent)
    compare_classes(original, reconstructed, &mut differences);

    // Compare slots (order-independent)
    compare_slots(original, reconstructed, &mut differences);

    // Compare enums (order-independent)
    compare_enums(original, reconstructed, &mut differences);

    if differences.is_empty() {
        EquivalenceResult::equivalent()
    } else {
        EquivalenceResult::different(differences)
    }
}

/// Compare schema metadata (id, name, description, etc.)
fn compare_schema_metadata(
    original: &SchemaDefinition,
    reconstructed: &SchemaDefinition,
    differences: &mut Vec<Difference>,
) {
    let path = "schema";

    // Compare schema name
    if original.name != reconstructed.name {
        differences.push(Difference::ValueMismatch {
            path: path.to_string(),
            field: "name".to_string(),
            expected: original.name.clone(),
            actual: reconstructed.name.clone(),
        });
    }

    // Compare schema id
    if original.id != reconstructed.id {
        differences.push(Difference::ValueMismatch {
            path: path.to_string(),
            field: "id".to_string(),
            expected: original.id.clone(),
            actual: reconstructed.id.clone(),
        });
    }

    // Compare description (optional, normalize whitespace)
    let orig_desc = original
        .description
        .as_ref()
        .map(|s| normalize_whitespace(s));
    let recon_desc = reconstructed
        .description
        .as_ref()
        .map(|s| normalize_whitespace(s));
    if orig_desc != recon_desc {
        differences.push(Difference::MetadataDifference {
            path: path.to_string(),
            field: "description".to_string(),
            expected: orig_desc,
            actual: recon_desc,
        });
    }
}

/// Compare classes in two schemas (order-independent)
fn compare_classes(
    original: &SchemaDefinition,
    reconstructed: &SchemaDefinition,
    differences: &mut Vec<Difference>,
) {
    // Get class names as sets for comparison
    let orig_classes: BTreeSet<_> = original.classes.keys().collect();
    let recon_classes: BTreeSet<_> = reconstructed.classes.keys().collect();

    // Check for missing classes
    for missing in orig_classes.difference(&recon_classes) {
        differences.push(Difference::MissingElement {
            path: "schema.classes".to_string(),
            element_type: "class".to_string(),
            name: (*missing).clone(),
        });
    }

    // Check for extra classes
    for extra in recon_classes.difference(&orig_classes) {
        differences.push(Difference::ExtraElement {
            path: "schema.classes".to_string(),
            element_type: "class".to_string(),
            name: (*extra).clone(),
        });
    }

    // Compare common classes
    for class_name in orig_classes.intersection(&recon_classes) {
        let orig_class = &original.classes[*class_name];
        let recon_class = &reconstructed.classes[*class_name];
        let class_path = format!("schema.classes.{class_name}");

        compare_class_definition(orig_class, recon_class, &class_path, differences);
    }
}

/// Compare individual class definitions
fn compare_class_definition(
    original: &ClassDefinition,
    reconstructed: &ClassDefinition,
    path: &str,
    differences: &mut Vec<Difference>,
) {
    // Compare class name
    if original.name != reconstructed.name {
        differences.push(Difference::ValueMismatch {
            path: path.to_string(),
            field: "name".to_string(),
            expected: original.name.clone(),
            actual: reconstructed.name.clone(),
        });
    }

    // Compare description (optional, normalize whitespace)
    let orig_desc = original
        .description
        .as_ref()
        .map(|s| normalize_whitespace(s));
    let recon_desc = reconstructed
        .description
        .as_ref()
        .map(|s| normalize_whitespace(s));
    if orig_desc != recon_desc {
        differences.push(Difference::MetadataDifference {
            path: path.to_string(),
            field: "description".to_string(),
            expected: orig_desc,
            actual: recon_desc,
        });
    }

    // Compare is_a relationships
    if original.is_a != reconstructed.is_a {
        differences.push(Difference::ValueMismatch {
            path: path.to_string(),
            field: "is_a".to_string(),
            expected: format!("{:?}", original.is_a),
            actual: format!("{:?}", reconstructed.is_a),
        });
    }

    // Compare attributes (order-independent)
    compare_attributes(
        &original.attributes,
        &reconstructed.attributes,
        path,
        differences,
    );

    // Compare mixins (order-independent set comparison)
    let orig_mixins: BTreeSet<_> = original.mixins.iter().collect();
    let recon_mixins: BTreeSet<_> = reconstructed.mixins.iter().collect();
    if orig_mixins != recon_mixins {
        differences.push(Difference::ValueMismatch {
            path: path.to_string(),
            field: "mixins".to_string(),
            expected: format!("{orig_mixins:?}"),
            actual: format!("{recon_mixins:?}"),
        });
    }
}

/// Compare attributes/slots in a class (order-independent)
fn compare_attributes(
    original: &IndexMap<String, SlotDefinition>,
    reconstructed: &IndexMap<String, SlotDefinition>,
    class_path: &str,
    differences: &mut Vec<Difference>,
) {
    let orig_attrs: BTreeSet<_> = original.keys().collect();
    let recon_attrs: BTreeSet<_> = reconstructed.keys().collect();

    // Check for missing attributes
    for missing in orig_attrs.difference(&recon_attrs) {
        differences.push(Difference::MissingElement {
            path: format!("{class_path}.attributes"),
            element_type: "attribute".to_string(),
            name: (*missing).clone(),
        });
    }

    // Check for extra attributes
    for extra in recon_attrs.difference(&orig_attrs) {
        differences.push(Difference::ExtraElement {
            path: format!("{class_path}.attributes"),
            element_type: "attribute".to_string(),
            name: (*extra).clone(),
        });
    }

    // Compare common attributes
    for attr_name in orig_attrs.intersection(&recon_attrs) {
        let orig_attr = &original[*attr_name];
        let recon_attr = &reconstructed[*attr_name];
        let attr_path = format!("{class_path}.attributes.{attr_name}");

        compare_slot_definition(orig_attr, recon_attr, &attr_path, differences);
    }
}

/// Compare individual slot definitions
fn compare_slot_definition(
    original: &SlotDefinition,
    reconstructed: &SlotDefinition,
    path: &str,
    differences: &mut Vec<Difference>,
) {
    // Compare slot name
    if original.name != reconstructed.name {
        differences.push(Difference::ValueMismatch {
            path: path.to_string(),
            field: "name".to_string(),
            expected: original.name.clone(),
            actual: reconstructed.name.clone(),
        });
    }

    // Compare range (type)
    if original.range != reconstructed.range {
        differences.push(Difference::TypeMismatch {
            path: path.to_string(),
            expected: original.range.as_deref().unwrap_or("<none>").to_string(),
            actual: reconstructed
                .range
                .as_deref()
                .unwrap_or("<none>")
                .to_string(),
        });
    }

    // Compare required constraint
    if original.required != reconstructed.required {
        differences.push(Difference::ConstraintViolation {
            path: path.to_string(),
            constraint_type: "required".to_string(),
            expected: format!("{:?}", original.required),
            actual: format!("{:?}", reconstructed.required),
        });
    }

    // Compare identifier flag
    if original.identifier != reconstructed.identifier {
        differences.push(Difference::ValueMismatch {
            path: path.to_string(),
            field: "identifier".to_string(),
            expected: format!("{:?}", original.identifier),
            actual: format!("{:?}", reconstructed.identifier),
        });
    }

    // Compare multivalued flag
    if original.multivalued != reconstructed.multivalued {
        differences.push(Difference::ValueMismatch {
            path: path.to_string(),
            field: "multivalued".to_string(),
            expected: format!("{:?}", original.multivalued),
            actual: format!("{:?}", reconstructed.multivalued),
        });
    }

    // Compare pattern constraint
    if original.pattern != reconstructed.pattern {
        differences.push(Difference::ConstraintViolation {
            path: path.to_string(),
            constraint_type: "pattern".to_string(),
            expected: original.pattern.as_deref().unwrap_or("<none>").to_string(),
            actual: reconstructed
                .pattern
                .as_deref()
                .unwrap_or("<none>")
                .to_string(),
        });
    }

    // Compare minimum_value constraint
    if original.minimum_value != reconstructed.minimum_value {
        differences.push(Difference::ConstraintViolation {
            path: path.to_string(),
            constraint_type: "minimum_value".to_string(),
            expected: format!("{:?}", original.minimum_value),
            actual: format!("{:?}", reconstructed.minimum_value),
        });
    }

    // Compare maximum_value constraint
    if original.maximum_value != reconstructed.maximum_value {
        differences.push(Difference::ConstraintViolation {
            path: path.to_string(),
            constraint_type: "maximum_value".to_string(),
            expected: format!("{:?}", original.maximum_value),
            actual: format!("{:?}", reconstructed.maximum_value),
        });
    }
}

/// Compare slots in two schemas (order-independent)
fn compare_slots(
    original: &SchemaDefinition,
    reconstructed: &SchemaDefinition,
    differences: &mut Vec<Difference>,
) {
    let orig_slots: BTreeSet<_> = original.slots.keys().collect();
    let recon_slots: BTreeSet<_> = reconstructed.slots.keys().collect();

    // Check for missing slots
    for missing in orig_slots.difference(&recon_slots) {
        differences.push(Difference::MissingElement {
            path: "schema.slots".to_string(),
            element_type: "slot".to_string(),
            name: (*missing).clone(),
        });
    }

    // Check for extra slots
    for extra in recon_slots.difference(&orig_slots) {
        differences.push(Difference::ExtraElement {
            path: "schema.slots".to_string(),
            element_type: "slot".to_string(),
            name: (*extra).clone(),
        });
    }

    // Compare common slots
    for slot_name in orig_slots.intersection(&recon_slots) {
        let orig_slot = &original.slots[*slot_name];
        let recon_slot = &reconstructed.slots[*slot_name];
        let slot_path = format!("schema.slots.{slot_name}");

        compare_slot_definition(orig_slot, recon_slot, &slot_path, differences);
    }
}

/// Compare enums in two schemas (order-independent)
fn compare_enums(
    original: &SchemaDefinition,
    reconstructed: &SchemaDefinition,
    differences: &mut Vec<Difference>,
) {
    let orig_enums: BTreeSet<_> = original.enums.keys().collect();
    let recon_enums: BTreeSet<_> = reconstructed.enums.keys().collect();

    // Check for missing enums
    for missing in orig_enums.difference(&recon_enums) {
        differences.push(Difference::MissingElement {
            path: "schema.enums".to_string(),
            element_type: "enum".to_string(),
            name: (*missing).clone(),
        });
    }

    // Check for extra enums
    for extra in recon_enums.difference(&orig_enums) {
        differences.push(Difference::ExtraElement {
            path: "schema.enums".to_string(),
            element_type: "enum".to_string(),
            name: (*extra).clone(),
        });
    }

    // Compare common enums
    for enum_name in orig_enums.intersection(&recon_enums) {
        let orig_enum = &original.enums[*enum_name];
        let recon_enum = &reconstructed.enums[*enum_name];
        let enum_path = format!("schema.enums.{enum_name}");

        compare_enum_definition(orig_enum, recon_enum, &enum_path, differences);
    }
}

/// Compare individual enum definitions
fn compare_enum_definition(
    original: &EnumDefinition,
    reconstructed: &EnumDefinition,
    path: &str,
    differences: &mut Vec<Difference>,
) {
    // Compare enum name
    if original.name != reconstructed.name {
        differences.push(Difference::ValueMismatch {
            path: path.to_string(),
            field: "name".to_string(),
            expected: original.name.clone(),
            actual: reconstructed.name.clone(),
        });
    }

    // Compare permissible values (order-independent)
    // permissible_values is now a Vec<PermissibleValue>, extract text values
    let orig_values: BTreeSet<_> = original
        .permissible_values
        .iter()
        .map(|pv| match pv {
            PermissibleValue::Simple(text) => text.as_str(),
            PermissibleValue::Complex { text, .. } => text.as_str(),
        })
        .collect();
    let recon_values: BTreeSet<_> = reconstructed
        .permissible_values
        .iter()
        .map(|pv| match pv {
            PermissibleValue::Simple(text) => text.as_str(),
            PermissibleValue::Complex { text, .. } => text.as_str(),
        })
        .collect();

    if orig_values != recon_values {
        differences.push(Difference::ValueMismatch {
            path: path.to_string(),
            field: "permissible_values".to_string(),
            expected: format!("{orig_values:?}"),
            actual: format!("{recon_values:?}"),
        });
    }
}

/// Normalize whitespace in strings for comparison
#[must_use]
fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_schemas_are_equivalent() {
        let schema1 = create_test_schema();
        let schema2 = create_test_schema();

        let result = compare_schemas(&schema1, &schema2);
        assert!(
            result.is_equivalent,
            "Identical schemas should be equivalent"
        );
    }

    #[test]
    fn test_different_schema_names_detected() {
        let mut schema1 = create_test_schema();
        let mut schema2 = create_test_schema();
        schema2.name = "different_name".to_string();

        let result = compare_schemas(&schema1, &schema2);
        assert!(
            !result.is_equivalent,
            "Different schema names should be detected"
        );
        assert_eq!(result.differences.len(), 1);
    }

    #[test]
    fn test_missing_class_detected() {
        let mut schema1 = create_test_schema();
        let schema2 = create_test_schema();

        // Add extra class to schema1
        let mut extra_class = ClassDefinition::new("ExtraClass");
        extra_class.name = "ExtraClass".to_string();
        schema1
            .classes
            .insert("ExtraClass".to_string(), extra_class);

        let result = compare_schemas(&schema1, &schema2);
        assert!(!result.is_equivalent, "Missing class should be detected");
        assert!(
            result
                .differences
                .iter()
                .any(|d| matches!(d, Difference::MissingElement { .. }))
        );
    }

    #[test]
    fn test_whitespace_normalization() {
        let s1 = "This   has   extra   spaces";
        let s2 = "This has extra spaces";
        assert_eq!(normalize_whitespace(s1), normalize_whitespace(s2));
    }

    fn create_test_schema() -> SchemaDefinition {
        let mut schema = SchemaDefinition::new("test_schema");
        schema.id = "test_schema".to_string();
        schema.name = "test_schema".to_string();

        let mut class = ClassDefinition::new("TestClass");
        class.name = "TestClass".to_string();
        schema.classes.insert("TestClass".to_string(), class);

        schema
    }
}
