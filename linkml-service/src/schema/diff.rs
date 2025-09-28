//! Schema diff functionality for `LinkML`
//!
//! This module provides tools to compare schemas and identify differences.

use linkml_core::annotations::{Annotatable, standard_annotations};
use linkml_core::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

/// Options for schema diffing
#[derive(Debug, Clone)]
pub struct DiffOptions {
    /// Include documentation changes
    pub include_documentation: bool,

    /// Show only breaking changes
    pub breaking_changes_only: bool,

    /// Number of context lines for unified diff
    pub context_lines: usize,
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self {
            include_documentation: true,
            breaking_changes_only: false,
            context_lines: 3,
        }
    }
}

/// Result of schema comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResult {
    /// Added classes
    pub added_classes: Vec<String>,

    /// Removed classes
    pub removed_classes: Vec<String>,

    /// Modified classes
    pub modified_classes: Vec<ClassDiff>,

    /// Added slots
    pub added_slots: Vec<String>,

    /// Removed slots
    pub removed_slots: Vec<String>,

    /// Modified slots
    pub modified_slots: Vec<SlotDiff>,

    /// Added types
    pub added_types: Vec<String>,

    /// Removed types
    pub removed_types: Vec<String>,

    /// Modified types
    pub modified_types: Vec<TypeDiff>,

    /// Added enums
    pub added_enums: Vec<String>,

    /// Removed enums
    pub removed_enums: Vec<String>,

    /// Modified enums
    pub modified_enums: Vec<EnumDiff>,

    /// Breaking changes detected
    pub breaking_changes: Vec<String>,
}

/// Class difference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassDiff {
    /// Class name
    pub name: String,

    /// Added slots
    pub added_slots: Vec<String>,

    /// Removed slots
    pub removed_slots: Vec<String>,

    /// Changed attributes
    pub changed_attributes: HashMap<String, AttributeChange>,
}

/// Slot difference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotDiff {
    /// Slot name
    pub name: String,

    /// Changed attributes
    pub changed_attributes: HashMap<String, AttributeChange>,
}

/// Type difference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDiff {
    /// Type name
    pub name: String,

    /// Changed attributes
    pub changed_attributes: HashMap<String, AttributeChange>,
}

/// Enum difference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumDiff {
    /// Enum name
    pub name: String,

    /// Added values
    pub added_values: Vec<String>,

    /// Removed values
    pub removed_values: Vec<String>,

    /// Changed attributes
    pub changed_attributes: HashMap<String, AttributeChange>,
}

/// Attribute change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeChange {
    /// Old value
    pub old_value: Option<serde_json::Value>,

    /// New value
    pub new_value: Option<serde_json::Value>,
}

/// Schema diff engine
pub struct SchemaDiff {
    options: DiffOptions,
}

impl SchemaDiff {
    /// Create new schema diff engine
    #[must_use]
    pub fn new(options: DiffOptions) -> Self {
        Self { options }
    }

    /// Compare two schemas
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn diff(
        &self,
        schema1: &SchemaDefinition,
        schema2: &SchemaDefinition,
    ) -> Result<DiffResult> {
        let mut result = DiffResult {
            added_classes: Vec::new(),
            removed_classes: Vec::new(),
            modified_classes: Vec::new(),
            added_slots: Vec::new(),
            removed_slots: Vec::new(),
            modified_slots: Vec::new(),
            added_types: Vec::new(),
            removed_types: Vec::new(),
            modified_types: Vec::new(),
            added_enums: Vec::new(),
            removed_enums: Vec::new(),
            modified_enums: Vec::new(),
            breaking_changes: Vec::new(),
        };

        // Compare classes
        self.compare_classes(schema1, schema2, &mut result)?;

        // Compare slots
        self.compare_slots(schema1, schema2, &mut result)?;

        // Compare types
        self.compare_types(schema1, schema2, &mut result)?;

        // Compare enums
        self.compare_enums(schema1, schema2, &mut result)?;

        // Detect breaking changes
        self.detect_breaking_changes(&result);

        Ok(result)
    }

    /// Compare classes between schemas
    fn compare_classes(
        &self,
        schema1: &SchemaDefinition,
        schema2: &SchemaDefinition,
        result: &mut DiffResult,
    ) -> Result<()> {
        let classes1: HashSet<_> = schema1.classes.keys().cloned().collect();
        let classes2: HashSet<_> = schema2.classes.keys().cloned().collect();

        // Added classes
        for class_name in classes2.difference(&classes1) {
            result.added_classes.push(class_name.clone());
        }

        // Removed classes
        for class_name in classes1.difference(&classes2) {
            result.removed_classes.push(class_name.clone());
            result
                .breaking_changes
                .push(format!("Class '{class_name}' was removed"));
        }

        // Modified classes
        for class_name in classes1.intersection(&classes2) {
            if let (Some(class1), Some(class2)) = (
                schema1.classes.get(class_name),
                schema2.classes.get(class_name),
            ) {
                let class_diff = self.compare_class(class_name, class1, class2)?;
                if !class_diff.added_slots.is_empty()
                    || !class_diff.removed_slots.is_empty()
                    || !class_diff.changed_attributes.is_empty()
                {
                    result.modified_classes.push(class_diff);
                }
            }
        }

        Ok(())
    }

    /// Compare individual class
    fn compare_class(
        &self,
        name: &str,
        class1: &ClassDefinition,
        class2: &ClassDefinition,
    ) -> Result<ClassDiff> {
        // Check if this class should be ignored in diff
        if class1.has_annotation(standard_annotations::IGNORE_IN_DIFF)
            || class2.has_annotation(standard_annotations::IGNORE_IN_DIFF)
        {
            return Ok(ClassDiff {
                name: name.to_string(),
                added_slots: Vec::new(),
                removed_slots: Vec::new(),
                changed_attributes: HashMap::new(),
            });
        }
        let mut diff = ClassDiff {
            name: name.to_string(),
            added_slots: Vec::new(),
            removed_slots: Vec::new(),
            changed_attributes: HashMap::new(),
        };

        let slots1: HashSet<_> = class1.slots.iter().cloned().collect();
        let slots2: HashSet<_> = class2.slots.iter().cloned().collect();

        // Added slots
        for slot in slots2.difference(&slots1) {
            diff.added_slots.push(slot.clone());
        }

        // Removed slots
        for slot in slots1.difference(&slots2) {
            diff.removed_slots.push(slot.clone());
        }

        // Compare attributes
        if class1.is_a != class2.is_a {
            diff.changed_attributes.insert(
                "is_a".to_string(),
                AttributeChange {
                    old_value: class1
                        .is_a
                        .as_ref()
                        .map(|v| serde_json::Value::String(v.clone())),
                    new_value: class2
                        .is_a
                        .as_ref()
                        .map(|v| serde_json::Value::String(v.clone())),
                },
            );
        }

        // Check if documentation changes should be ignored
        let ignore_docs = class1.has_annotation(standard_annotations::IGNORE_DOCS_IN_DIFF)
            || class2.has_annotation(standard_annotations::IGNORE_DOCS_IN_DIFF);

        if self.options.include_documentation
            && !ignore_docs
            && class1.description != class2.description
        {
            diff.changed_attributes.insert(
                "description".to_string(),
                AttributeChange {
                    old_value: class1
                        .description
                        .as_ref()
                        .map(|v| serde_json::Value::String(v.clone())),
                    new_value: class2
                        .description
                        .as_ref()
                        .map(|v| serde_json::Value::String(v.clone())),
                },
            );
        }

        Ok(diff)
    }

    /// Compare slots between schemas
    fn compare_slots(
        &self,
        schema1: &SchemaDefinition,
        schema2: &SchemaDefinition,
        result: &mut DiffResult,
    ) -> Result<()> {
        // Filter out slots marked with ignore_in_diff
        let should_compare_slot = |slot: &SlotDefinition| -> bool {
            !slot.has_annotation(standard_annotations::IGNORE_IN_DIFF)
        };
        let slots1: HashSet<_> = schema1.slots.keys().cloned().collect();
        let slots2: HashSet<_> = schema2.slots.keys().cloned().collect();

        // Added slots
        for slot_name in slots2.difference(&slots1) {
            if let Some(slot) = schema2.slots.get(slot_name)
                && should_compare_slot(slot)
            {
                result.added_slots.push(slot_name.clone());
            }
        }

        // Removed slots
        for slot_name in slots1.difference(&slots2) {
            if let Some(slot) = schema1.slots.get(slot_name)
                && should_compare_slot(slot)
            {
                result.removed_slots.push(slot_name.clone());
                // Check if this is marked as a breaking change
                if slot.has_annotation(standard_annotations::BREAKING_IF_CHANGED) {
                    result
                        .breaking_changes
                        .push(format!("Breaking: Slot '{slot_name}' was removed"));
                }
            }
        }

        // Modified slots
        for slot_name in slots1.intersection(&slots2) {
            if let (Some(slot1), Some(slot2)) =
                (schema1.slots.get(slot_name), schema2.slots.get(slot_name))
            {
                let slot_diff = self.compare_slot(slot_name, slot1, slot2)?;
                if !slot_diff.changed_attributes.is_empty() {
                    result.modified_slots.push(slot_diff);
                }
            }
        }

        Ok(())
    }

    /// Compare individual slot
    fn compare_slot(
        &self,
        name: &str,
        slot1: &SlotDefinition,
        slot2: &SlotDefinition,
    ) -> Result<SlotDiff> {
        let mut diff = SlotDiff {
            name: name.to_string(),
            changed_attributes: HashMap::new(),
        };

        // Compare range
        if slot1.range != slot2.range {
            diff.changed_attributes.insert(
                "range".to_string(),
                AttributeChange {
                    old_value: slot1
                        .range
                        .as_ref()
                        .map(|v| serde_json::Value::String(v.clone())),
                    new_value: slot2
                        .range
                        .as_ref()
                        .map(|v| serde_json::Value::String(v.clone())),
                },
            );
        }

        // Compare required
        if slot1.required != slot2.required {
            diff.changed_attributes.insert(
                "required".to_string(),
                AttributeChange {
                    old_value: slot1.required.map(serde_json::Value::Bool),
                    new_value: slot2.required.map(serde_json::Value::Bool),
                },
            );
        }

        // Compare multivalued
        if slot1.multivalued != slot2.multivalued {
            diff.changed_attributes.insert(
                "multivalued".to_string(),
                AttributeChange {
                    old_value: slot1.multivalued.map(serde_json::Value::Bool),
                    new_value: slot2.multivalued.map(serde_json::Value::Bool),
                },
            );
        }

        Ok(diff)
    }

    /// Compare types between schemas
    fn compare_types(
        &self,
        schema1: &SchemaDefinition,
        schema2: &SchemaDefinition,
        result: &mut DiffResult,
    ) -> Result<()> {
        let types1: HashSet<_> = schema1.types.keys().cloned().collect();
        let types2: HashSet<_> = schema2.types.keys().cloned().collect();

        // Added types
        for type_name in types2.difference(&types1) {
            result.added_types.push(type_name.clone());
        }

        // Removed types
        for type_name in types1.difference(&types2) {
            result.removed_types.push(type_name.clone());
            result
                .breaking_changes
                .push(format!("Type '{type_name}' was removed"));
        }

        // Modified types
        for type_name in types1.intersection(&types2) {
            if let (Some(type1), Some(type2)) =
                (schema1.types.get(type_name), schema2.types.get(type_name))
            {
                let type_diff = self.compare_type(type_name, type1, type2)?;
                if !type_diff.changed_attributes.is_empty() {
                    result.modified_types.push(type_diff);
                }
            }
        }

        Ok(())
    }

    /// Compare individual type
    fn compare_type(
        &self,
        name: &str,
        type1: &TypeDefinition,
        type2: &TypeDefinition,
    ) -> Result<TypeDiff> {
        let mut diff = TypeDiff {
            name: name.to_string(),
            changed_attributes: HashMap::new(),
        };

        // Compare base_type
        if type1.base_type != type2.base_type {
            diff.changed_attributes.insert(
                "base_type".to_string(),
                AttributeChange {
                    old_value: type1
                        .base_type
                        .as_ref()
                        .map(|v| serde_json::Value::String(v.clone())),
                    new_value: type2
                        .base_type
                        .as_ref()
                        .map(|v| serde_json::Value::String(v.clone())),
                },
            );
        }

        Ok(diff)
    }

    /// Compare enums between schemas
    fn compare_enums(
        &self,
        schema1: &SchemaDefinition,
        schema2: &SchemaDefinition,
        result: &mut DiffResult,
    ) -> Result<()> {
        let enums1: HashSet<_> = schema1.enums.keys().cloned().collect();
        let enums2: HashSet<_> = schema2.enums.keys().cloned().collect();

        // Added enums
        for enum_name in enums2.difference(&enums1) {
            result.added_enums.push(enum_name.clone());
        }

        // Removed enums
        for enum_name in enums1.difference(&enums2) {
            result.removed_enums.push(enum_name.clone());
            result
                .breaking_changes
                .push(format!("Enum '{enum_name}' was removed"));
        }

        // Modified enums
        for enum_name in enums1.intersection(&enums2) {
            if let (Some(enum1), Some(enum2)) =
                (schema1.enums.get(enum_name), schema2.enums.get(enum_name))
            {
                let enum_diff = self.compare_enum(enum_name, enum1, enum2)?;
                if !enum_diff.added_values.is_empty()
                    || !enum_diff.removed_values.is_empty()
                    || !enum_diff.changed_attributes.is_empty()
                {
                    result.modified_enums.push(enum_diff);
                }
            }
        }

        Ok(())
    }

    /// Compare individual enum
    fn compare_enum(
        &self,
        name: &str,
        enum1: &EnumDefinition,
        enum2: &EnumDefinition,
    ) -> Result<EnumDiff> {
        let mut diff = EnumDiff {
            name: name.to_string(),
            added_values: Vec::new(),
            removed_values: Vec::new(),
            changed_attributes: HashMap::new(),
        };

        let values1: HashSet<_> = enum1
            .permissible_values
            .iter()
            .map(|pv| match pv {
                linkml_core::types::PermissibleValue::Simple(s) => s.clone(),
                linkml_core::types::PermissibleValue::Complex { text, .. } => text.clone(),
            })
            .collect();
        let values2: HashSet<_> = enum2
            .permissible_values
            .iter()
            .map(|pv| match pv {
                linkml_core::types::PermissibleValue::Simple(s) => s.clone(),
                linkml_core::types::PermissibleValue::Complex { text, .. } => text.clone(),
            })
            .collect();

        // Added values
        for value in values2.difference(&values1) {
            diff.added_values.push(value.clone());
        }

        // Removed values
        for value in values1.difference(&values2) {
            diff.removed_values.push(value.clone());
        }

        Ok(diff)
    }

    /// Detect breaking changes
    fn detect_breaking_changes(&self, _result: &DiffResult) {
        // Already handled during comparison
        // Additional breaking change detection could go here
    }
}

impl DiffResult {
    /// Convert to unified diff format
    #[must_use]
    pub fn to_unified_diff(&self) -> String {
        let mut output = String::new();

        // Header
        output.push_str(
            "--- Schema 1
",
        );
        output.push_str(
            "+++ Schema 2
",
        );
        output.push('\n');

        // Classes
        if !self.added_classes.is_empty()
            || !self.removed_classes.is_empty()
            || !self.modified_classes.is_empty()
        {
            output.push_str(
                "@@ Classes @@
",
            );

            for class in &self.removed_classes {
                writeln!(output, "- class: {class}").expect("writeln! to String should never fail");
            }

            for class in &self.added_classes {
                writeln!(output, "+ class: {class}").expect("writeln! to String should never fail");
            }

            for class_diff in &self.modified_classes {
                writeln!(output, "  class: {}", class_diff.name)
                    .expect("writeln! to String should never fail");

                for slot in &class_diff.removed_slots {
                    writeln!(output, "    - slot: {slot}")
                        .expect("writeln! to String should never fail");
                }

                for slot in &class_diff.added_slots {
                    writeln!(output, "    + slot: {slot}")
                        .expect("writeln! to String should never fail");
                }
            }

            output.push('\n');
        }

        // Slots
        if !self.added_slots.is_empty()
            || !self.removed_slots.is_empty()
            || !self.modified_slots.is_empty()
        {
            output.push_str(
                "@@ Slots @@
",
            );

            for slot in &self.removed_slots {
                writeln!(output, "- slot: {slot}").expect("writeln! to String should never fail");
            }

            for slot in &self.added_slots {
                writeln!(output, "+ slot: {slot}").expect("writeln! to String should never fail");
            }

            for slot_diff in &self.modified_slots {
                writeln!(output, "  slot: {}", slot_diff.name)
                    .expect("writeln! to String should never fail");

                for (attr, change) in &slot_diff.changed_attributes {
                    writeln!(
                        output,
                        "    {}: {:?} -> {:?}",
                        attr, change.old_value, change.new_value
                    )
                    .expect("LinkML operation should succeed");
                }
            }

            output.push('\n');
        }

        output
    }

    /// Convert to side-by-side diff format
    #[must_use]
    pub fn to_side_by_side(&self) -> String {
        let mut output = String::new();

        writeln!(output, "{:<40} | {:<40}", "Schema 1", "Schema 2")
            .expect("writeln! to String should never fail");
        writeln!(output, "{:-<40} | {:-<40}", "", "")
            .expect("writeln! to String should never fail");

        // Classes
        output.push_str(
            "
Classes:
",
        );
        for class in &self.removed_classes {
            writeln!(output, "{:<40} | {:<40}", class, "")
                .expect("writeln! to String should never fail");
        }
        for class in &self.added_classes {
            writeln!(output, "{:<40} | {:<40}", "", class)
                .expect("writeln! to String should never fail");
        }

        output
    }

    /// Convert to `JSON` patch format
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn to_json_patch(&self) -> Result<String> {
        let mut patches = Vec::new();

        // Add operations for added classes
        for class in &self.added_classes {
            patches.push(serde_json::json!({
                "op": "add",
                "path": format!("/classes/{class}"),
                "value": {}
            }));
        }

        // Remove operations for removed classes
        for class in &self.removed_classes {
            patches.push(serde_json::json!({
                "op": "remove",
                "path": format!("/classes/{class}")
            }));
        }

        Ok(serde_json::to_string_pretty(&patches)?)
    }

    /// Convert to HTML diff
    #[must_use]
    pub fn to_html(&self) -> String {
        let mut html = String::new();

        html.push_str(
            "<!DOCTYPE html>
<html>
<head>
",
        );
        html.push_str(
            "<style>
",
        );
        html.push_str(
            ".added { background-color: #e6ffed; }
",
        );
        html.push_str(
            ".removed { background-color: #ffeef0; }
",
        );
        html.push_str(
            ".modified { background-color: #fff5b1; }
",
        );
        html.push_str(
            "</style>
</head>
<body>
",
        );

        html.push_str(
            "<h1>Schema Diff</h1>
",
        );

        // Summary
        html.push_str(
            "<h2>Summary</h2>
",
        );
        html.push_str(
            "<ul>
",
        );
        use std::fmt::Write;
        write!(
            html,
            "<li>Added: {} classes, {} slots</li>
",
            self.added_classes.len(),
            self.added_slots.len()
        )
        .expect("write! to String should never fail");
        write!(
            html,
            "<li>Removed: {} classes, {} slots</li>
",
            self.removed_classes.len(),
            self.removed_slots.len()
        )
        .expect("write! to String should never fail");
        write!(
            html,
            "<li>Modified: {} classes, {} slots</li>
",
            self.modified_classes.len(),
            self.modified_slots.len()
        )
        .expect("write! to String should never fail");
        html.push_str(
            "</ul>
",
        );

        // Details
        if !self.added_classes.is_empty() {
            html.push_str(
                "<h2>Added Classes</h2>
<ul>
",
            );
            for class in &self.added_classes {
                writeln!(html, "<li class='added'>{class}</li>")
                    .expect("writeln! to String should never fail");
            }
            html.push_str(
                "</ul>
",
            );
        }

        if !self.removed_classes.is_empty() {
            html.push_str(
                "<h2>Removed Classes</h2>
<ul>
",
            );
            for class in &self.removed_classes {
                writeln!(html, "<li class='removed'>{class}</li>")
                    .expect("writeln! to String should never fail");
            }
            html.push_str(
                "</ul>
",
            );
        }

        html.push_str(
            "</body>
</html>",
        );

        html
    }

    /// Convert to Markdown diff
    #[must_use]
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str(
            "# Schema Diff

",
        );

        // Summary
        md.push_str(
            "## Summary

",
        );
        write!(
            md,
            "- **Added**: {} classes, {} slots
",
            self.added_classes.len(),
            self.added_slots.len()
        )
        .expect("write! to String should never fail");
        write!(
            md,
            "- **Removed**: {} classes, {} slots
",
            self.removed_classes.len(),
            self.removed_slots.len()
        )
        .expect("write! to String should never fail");
        write!(
            md,
            "- **Modified**: {} classes, {} slots

",
            self.modified_classes.len(),
            self.modified_slots.len()
        )
        .expect("write! to String should never fail");

        // Breaking changes
        if !self.breaking_changes.is_empty() {
            md.push_str(
                "## ⚠️ Breaking Changes

",
            );
            for change in &self.breaking_changes {
                writeln!(md, "- {change}").expect("writeln! to String should never fail");
            }
            md.push('\n');
        }

        // Added classes
        if !self.added_classes.is_empty() {
            md.push_str(
                "## Added Classes

",
            );
            for class in &self.added_classes {
                writeln!(md, "- ✅ `{class}`").expect("writeln! to String should never fail");
            }
            md.push('\n');
        }

        // Removed classes
        if !self.removed_classes.is_empty() {
            md.push_str(
                "## Removed Classes

",
            );
            for class in &self.removed_classes {
                writeln!(md, "- ❌ `{class}`").expect("writeln! to String should never fail");
            }
            md.push('\n');
        }

        // Modified classes
        if !self.modified_classes.is_empty() {
            md.push_str(
                "## Modified Classes

",
            );
            for class_diff in &self.modified_classes {
                writeln!(
                    md,
                    "### `{}``
",
                    class_diff.name
                )
                .expect("writeln! to String should never fail");

                if !class_diff.added_slots.is_empty() {
                    md.push_str(
                        "**Added slots:**
",
                    );
                    for slot in &class_diff.added_slots {
                        writeln!(md, "- ✅ `{slot}`")
                            .expect("writeln! to String should never fail");
                    }
                    md.push('\n');
                }

                if !class_diff.removed_slots.is_empty() {
                    md.push_str(
                        "**Removed slots:**
",
                    );
                    for slot in &class_diff.removed_slots {
                        writeln!(md, "- ❌ `{slot}`")
                            .expect("writeln! to String should never fail");
                    }
                    md.push('\n');
                }
            }
        }

        md
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{ClassDefinition, SchemaDefinition};

    #[test]
    fn test_schema_diff_basic() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut schema1 = SchemaDefinition::default();
        schema1.name = "TestSchema".to_string();

        let mut class1 = ClassDefinition::default();
        class1.slots = vec!["name".to_string(), "age".to_string()];
        schema1.classes.insert("Person".to_string(), class1);

        let mut schema2 = schema1.clone();

        // Add a new class
        let mut class2 = ClassDefinition::default();
        class2.slots = vec!["brand".to_string()];
        schema2.classes.insert("Car".to_string(), class2);

        // Modify existing class
        schema2
            .classes
            .get_mut("Person")
            .ok_or_else(|| anyhow::anyhow!("Person class should exist"))?
            .slots
            .push("email".to_string());

        let differ = SchemaDiff::new(DiffOptions::default());
        let result = differ
            .diff(&schema1, &schema2)
            .expect("should diff schemas: {}");

        assert_eq!(result.added_classes, vec!["Car"]);
        assert!(result.removed_classes.is_empty());
        assert_eq!(result.modified_classes.len(), 1);
        assert_eq!(result.modified_classes[0].name, "Person");
        assert_eq!(result.modified_classes[0].added_slots, vec!["email"]);
        Ok(())
    }
}
