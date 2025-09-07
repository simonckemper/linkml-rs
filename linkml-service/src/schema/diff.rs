//! Schema diff functionality for LinkML
//!
//! This module provides tools to compare schemas and identify differences.

use linkml_core::annotations::{Annotatable, standard_annotations};
use linkml_core::error::Result;
use linkml_core::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

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
    pub fn new(options: DiffOptions) -> Self {
        Self { options }
    }

    /// Compare two schemas
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
            if let Some(slot) = schema2.slots.get(slot_name) {
                if should_compare_slot(slot) {
                    result.added_slots.push(slot_name.clone());
                }
            }
        }

        // Removed slots
        for slot_name in slots1.difference(&slots2) {
            if let Some(slot) = schema1.slots.get(slot_name) {
                if should_compare_slot(slot) {
                    result.removed_slots.push(slot_name.clone());
                    // Check if this is marked as a breaking change
                    if slot.has_annotation(standard_annotations::BREAKING_IF_CHANGED) {
                        result
                            .breaking_changes
                            .push(format!("Breaking: Slot '{slot_name}' was removed"));
                    }
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
    pub fn to_unified_diff(&self) -> String {
        let mut output = String::new();

        // Header
        output.push_str("--- Schema 1\n");
        output.push_str("+++ Schema 2\n");
        output.push_str("\n");

        // Classes
        if !self.added_classes.is_empty()
            || !self.removed_classes.is_empty()
            || !self.modified_classes.is_empty()
        {
            output.push_str("@@ Classes @@\n");

            for class in &self.removed_classes {
                output.push_str(&format!("- class: {class}\n"));
            }

            for class in &self.added_classes {
                output.push_str(&format!("+ class: {class}\n"));
            }

            for class_diff in &self.modified_classes {
                output.push_str(&format!("  class: {}\n", class_diff.name));

                for slot in &class_diff.removed_slots {
                    output.push_str(&format!("    - slot: {slot}\n"));
                }

                for slot in &class_diff.added_slots {
                    output.push_str(&format!("    + slot: {slot}\n"));
                }
            }

            output.push_str("\n");
        }

        // Slots
        if !self.added_slots.is_empty()
            || !self.removed_slots.is_empty()
            || !self.modified_slots.is_empty()
        {
            output.push_str("@@ Slots @@\n");

            for slot in &self.removed_slots {
                output.push_str(&format!("- slot: {slot}\n"));
            }

            for slot in &self.added_slots {
                output.push_str(&format!("+ slot: {slot}\n"));
            }

            for slot_diff in &self.modified_slots {
                output.push_str(&format!("  slot: {}\n", slot_diff.name));

                for (attr, change) in &slot_diff.changed_attributes {
                    output.push_str(&format!(
                        "    {}: {:?} -> {:?}\n",
                        attr, change.old_value, change.new_value
                    ));
                }
            }

            output.push_str("\n");
        }

        output
    }

    /// Convert to side-by-side diff format
    pub fn to_side_by_side(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!("{:<40} | {:<40}\n", "Schema 1", "Schema 2"));
        output.push_str(&format!("{:-<40} | {:-<40}\n", "", ""));

        // Classes
        output.push_str("\nClasses:\n");
        for class in &self.removed_classes {
            output.push_str(&format!("{:<40} | {:<40}\n", class, ""));
        }
        for class in &self.added_classes {
            output.push_str(&format!("{:<40} | {:<40}\n", "", class));
        }

        output
    }

    /// Convert to `JSON` patch format
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
    pub fn to_html(&self) -> String {
        let mut html = String::new();

        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        html.push_str("<style>\n");
        html.push_str(".added { background-color: #e6ffed; }\n");
        html.push_str(".removed { background-color: #ffeef0; }\n");
        html.push_str(".modified { background-color: #fff5b1; }\n");
        html.push_str("</style>\n</head>\n<body>\n");

        html.push_str("<h1>Schema Diff</h1>\n");

        // Summary
        html.push_str("<h2>Summary</h2>\n");
        html.push_str("<ul>\n");
        html.push_str(&format!(
            "<li>Added: {} classes, {} slots</li>\n",
            self.added_classes.len(),
            self.added_slots.len()
        ));
        html.push_str(&format!(
            "<li>Removed: {} classes, {} slots</li>\n",
            self.removed_classes.len(),
            self.removed_slots.len()
        ));
        html.push_str(&format!(
            "<li>Modified: {} classes, {} slots</li>\n",
            self.modified_classes.len(),
            self.modified_slots.len()
        ));
        html.push_str("</ul>\n");

        // Details
        if !self.added_classes.is_empty() {
            html.push_str("<h2>Added Classes</h2>\n<ul>\n");
            for class in &self.added_classes {
                html.push_str(&format!("<li class='added'>{class}</li>\n"));
            }
            html.push_str("</ul>\n");
        }

        if !self.removed_classes.is_empty() {
            html.push_str("<h2>Removed Classes</h2>\n<ul>\n");
            for class in &self.removed_classes {
                html.push_str(&format!("<li class='removed'>{class}</li>\n"));
            }
            html.push_str("</ul>\n");
        }

        html.push_str("</body>\n</html>");

        html
    }

    /// Convert to Markdown diff
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str("# Schema Diff\n\n");

        // Summary
        md.push_str("## Summary\n\n");
        md.push_str(&format!(
            "- **Added**: {} classes, {} slots\n",
            self.added_classes.len(),
            self.added_slots.len()
        ));
        md.push_str(&format!(
            "- **Removed**: {} classes, {} slots\n",
            self.removed_classes.len(),
            self.removed_slots.len()
        ));
        md.push_str(&format!(
            "- **Modified**: {} classes, {} slots\n\n",
            self.modified_classes.len(),
            self.modified_slots.len()
        ));

        // Breaking changes
        if !self.breaking_changes.is_empty() {
            md.push_str("## ⚠️ Breaking Changes\n\n");
            for change in &self.breaking_changes {
                md.push_str(&format!("- {change}\n"));
            }
            md.push_str("\n");
        }

        // Added classes
        if !self.added_classes.is_empty() {
            md.push_str("## Added Classes\n\n");
            for class in &self.added_classes {
                md.push_str(&format!("- ✅ `{class}`\n"));
            }
            md.push_str("\n");
        }

        // Removed classes
        if !self.removed_classes.is_empty() {
            md.push_str("## Removed Classes\n\n");
            for class in &self.removed_classes {
                md.push_str(&format!("- ❌ `{class}`\n"));
            }
            md.push_str("\n");
        }

        // Modified classes
        if !self.modified_classes.is_empty() {
            md.push_str("## Modified Classes\n\n");
            for class_diff in &self.modified_classes {
                md.push_str(&format!("### `{}`\n\n", class_diff.name));

                if !class_diff.added_slots.is_empty() {
                    md.push_str("**Added slots:**\n");
                    for slot in &class_diff.added_slots {
                        md.push_str(&format!("- ✅ `{slot}`\n"));
                    }
                    md.push_str("\n");
                }

                if !class_diff.removed_slots.is_empty() {
                    md.push_str("**Removed slots:**\n");
                    for slot in &class_diff.removed_slots {
                        md.push_str(&format!("- ❌ `{slot}`\n"));
                    }
                    md.push_str("\n");
                }
            }
        }

        md
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_diff_basic() {
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
            .map_err(|e| anyhow::anyhow!("should diff schemas: {}", e))?;

        assert_eq!(result.added_classes, vec!["Car"]);
        assert!(result.removed_classes.is_empty());
        assert_eq!(result.modified_classes.len(), 1);
        assert_eq!(result.modified_classes[0].name, "Person");
        assert_eq!(result.modified_classes[0].added_slots, vec!["email"]);
    }
}
