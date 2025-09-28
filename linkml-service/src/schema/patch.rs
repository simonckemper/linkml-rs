//! Schema patch functionality for `LinkML`
//!
//! This module provides tools to apply patches to schemas, enabling
//! controlled schema evolution and migration.

use linkml_core::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::diff::DiffResult;

/// A patch operation to apply to a schema
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum PatchOperation {
    /// Add a new element
    #[serde(rename = "add")]
    Add {
        /// JSON Pointer path where the element should be added
        path: String,
        /// The value to add at the specified path
        value: Value,
    },
    /// Remove an element
    #[serde(rename = "remove")]
    Remove {
        /// JSON Pointer path of the element to remove
        path: String,
    },
    /// Replace an element
    #[serde(rename = "replace")]
    Replace {
        /// JSON Pointer path of the element to replace
        path: String,
        /// The new value to replace the existing element with
        value: Value,
    },
    /// Move an element
    #[serde(rename = "move")]
    Move {
        /// JSON Pointer path of the element to move from
        from: String,
        /// JSON Pointer path where the element should be moved to
        path: String,
    },
    /// Copy an element
    #[serde(rename = "copy")]
    Copy {
        /// JSON Pointer path of the element to copy from
        from: String,
        /// JSON Pointer path where the element should be copied to
        path: String,
    },
    /// Test a value (for conditional patches)
    #[serde(rename = "test")]
    Test {
        /// JSON Pointer path of the element to test
        path: String,
        /// The expected value at the specified path
        value: Value,
    },
}

/// A collection of patch operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaPatch {
    /// Patch operations to apply
    pub operations: Vec<PatchOperation>,

    /// Optional description of the patch
    pub description: Option<String>,

    /// Version this patch migrates from
    pub from_version: Option<String>,

    /// Version this patch migrates to
    pub to_version: Option<String>,

    /// Whether this is a breaking change
    pub breaking: bool,
}

/// Options for applying patches
#[derive(Debug, Clone)]
pub struct PatchOptions {
    /// Validate schema after each operation
    pub validate_after_each: bool,

    /// Allow breaking changes
    pub allow_breaking: bool,

    /// Create backup before applying
    pub create_backup: bool,

    /// Strict mode - fail on any warning
    pub strict: bool,
}

impl Default for PatchOptions {
    fn default() -> Self {
        Self {
            validate_after_each: false,
            allow_breaking: false,
            create_backup: true,
            strict: false,
        }
    }
}

/// Result of applying a patch
#[derive(Debug)]
pub struct PatchResult {
    /// The schema after applying the patch operations
    pub schema: SchemaDefinition,

    /// Operations that were applied
    pub applied_operations: Vec<PatchOperation>,

    /// Operations that were skipped
    pub skipped_operations: Vec<(PatchOperation, String)>,

    /// Warnings generated during patching
    pub warnings: Vec<String>,
}

/// Schema patcher for applying patches to schemas
pub struct SchemaPatcher {
    options: PatchOptions,
}

impl SchemaPatcher {
    /// Create a new schema patcher
    #[must_use]
    pub fn new(options: PatchOptions) -> Self {
        Self { options }
    }

    /// Apply a patch to a schema
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn apply_patch(
        &self,
        mut schema: SchemaDefinition,
        patch: &SchemaPatch,
    ) -> Result<PatchResult> {
        // Check if breaking changes are allowed
        if patch.breaking && !self.options.allow_breaking {
            return Err(LinkMLError::config(
                "Patch contains breaking changes but allow_breaking is false",
            ));
        }

        let mut result = PatchResult {
            schema: schema.clone(),
            applied_operations: Vec::new(),
            skipped_operations: Vec::new(),
            warnings: Vec::new(),
        };

        // Apply each operation
        for operation in &patch.operations {
            match self.apply_operation(&mut schema, operation) {
                Ok(()) => {
                    result.applied_operations.push(operation.clone());

                    // Validate if requested
                    if self.options.validate_after_each
                        && let Err(e) = self.validate_schema(&schema)
                    {
                        result
                            .warnings
                            .push(format!("Schema validation warning after operation: {e}"));
                        if self.options.strict {
                            return Err(e);
                        }
                    }
                }
                Err(e) => {
                    if self.options.strict {
                        return Err(e);
                    }
                    result
                        .skipped_operations
                        .push((operation.clone(), e.to_string()));
                }
            }
        }

        result.schema = schema;
        Ok(result)
    }

    /// Apply a single patch operation
    fn apply_operation(
        &self,
        schema: &mut SchemaDefinition,
        operation: &PatchOperation,
    ) -> Result<()> {
        match operation {
            PatchOperation::Add { path, value } => self.apply_add(schema, path, value),
            PatchOperation::Remove { path } => self.apply_remove(schema, path),
            PatchOperation::Replace { path, value } => self.apply_replace(schema, path, value),
            PatchOperation::Move { from, path } => self.apply_move(schema, from, path),
            PatchOperation::Copy { from, path } => self.apply_copy(schema, from, path),
            PatchOperation::Test { path, value } => self.apply_test(schema, path, value),
        }
    }

    /// Apply an add operation
    fn apply_add(&self, schema: &mut SchemaDefinition, path: &str, value: &Value) -> Result<()> {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

        match parts.as_slice() {
            ["classes", class_name] => {
                let class_def: ClassDefinition = serde_json::from_value(value.clone())
                    .map_err(|e| LinkMLError::parse(format!("Invalid class definition: {e}")))?;
                schema.classes.insert((*class_name).to_string(), class_def);
                Ok(())
            }
            ["classes", class_name, "slots", slot_name] => {
                if let Some(class) = schema.classes.get_mut(*class_name) {
                    class.slots.push((*slot_name).to_string());
                    Ok(())
                } else {
                    Err(LinkMLError::service(format!(
                        "Class '{class_name}' not found"
                    )))
                }
            }
            ["slots", slot_name] => {
                let slot_def: SlotDefinition = serde_json::from_value(value.clone())
                    .map_err(|e| LinkMLError::parse(format!("Invalid slot definition: {e}")))?;
                schema.slots.insert((*slot_name).to_string(), slot_def);
                Ok(())
            }
            ["types", type_name] => {
                let type_def: TypeDefinition = serde_json::from_value(value.clone())
                    .map_err(|e| LinkMLError::parse(format!("Invalid type definition: {e}")))?;
                schema.types.insert((*type_name).to_string(), type_def);
                Ok(())
            }
            ["enums", enum_name] => {
                let enum_def: EnumDefinition = serde_json::from_value(value.clone())
                    .map_err(|e| LinkMLError::parse(format!("Invalid enum definition: {e}")))?;
                schema.enums.insert((*enum_name).to_string(), enum_def);
                Ok(())
            }
            _ => Err(LinkMLError::service(format!(
                "Unsupported path for add: {path}"
            ))),
        }
    }

    /// Apply a remove operation
    fn apply_remove(&self, schema: &mut SchemaDefinition, path: &str) -> Result<()> {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

        match parts.as_slice() {
            ["classes", class_name] => {
                schema.classes.shift_remove(*class_name).ok_or_else(|| {
                    LinkMLError::service(format!("Class '{class_name}' not found"))
                })?;
                Ok(())
            }
            ["classes", class_name, "slots", slot_name] => {
                if let Some(class) = schema.classes.get_mut(*class_name) {
                    class.slots.retain(|s| s != slot_name);
                    Ok(())
                } else {
                    Err(LinkMLError::service(format!(
                        "Class '{class_name}' not found"
                    )))
                }
            }
            ["slots", slot_name] => {
                schema
                    .slots
                    .shift_remove(*slot_name)
                    .ok_or_else(|| LinkMLError::service(format!("Slot '{slot_name}' not found")))?;
                Ok(())
            }
            ["types", type_name] => {
                schema
                    .types
                    .shift_remove(*type_name)
                    .ok_or_else(|| LinkMLError::service(format!("Type '{type_name}' not found")))?;
                Ok(())
            }
            ["enums", enum_name] => {
                schema
                    .enums
                    .shift_remove(*enum_name)
                    .ok_or_else(|| LinkMLError::service(format!("Enum '{enum_name}' not found")))?;
                Ok(())
            }
            _ => Err(LinkMLError::service(format!(
                "Unsupported path for remove: {path}"
            ))),
        }
    }

    /// Apply a replace operation
    fn apply_replace(
        &self,
        schema: &mut SchemaDefinition,
        path: &str,
        value: &Value,
    ) -> Result<()> {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

        match parts.as_slice() {
            ["classes", class_name] => {
                let class_def: ClassDefinition = serde_json::from_value(value.clone())
                    .map_err(|e| LinkMLError::parse(format!("Invalid class definition: {e}")))?;
                schema.classes.insert((*class_name).to_string(), class_def);
                Ok(())
            }
            ["classes", class_name, "description"] => {
                if let Some(class) = schema.classes.get_mut(*class_name) {
                    class.description = value.as_str().map(std::string::ToString::to_string);
                    Ok(())
                } else {
                    Err(LinkMLError::service(format!(
                        "Class '{class_name}' not found"
                    )))
                }
            }
            ["slots", slot_name, "required"] => {
                if let Some(slot) = schema.slots.get_mut(*slot_name) {
                    slot.required = value.as_bool();
                    Ok(())
                } else {
                    Err(LinkMLError::service(format!(
                        "Slot '{slot_name}' not found"
                    )))
                }
            }
            _ => Err(LinkMLError::service(format!(
                "Unsupported path for replace: {path}"
            ))),
        }
    }

    /// Apply a move operation
    fn apply_move(&self, schema: &mut SchemaDefinition, from: &str, to: &str) -> Result<()> {
        // Extract value from source
        let value = self.extract_value(schema, from)?;

        // Remove from source
        self.apply_remove(schema, from)?;

        // Add to destination
        self.apply_add(schema, to, &value)?;

        Ok(())
    }

    /// Apply a copy operation
    fn apply_copy(&self, schema: &mut SchemaDefinition, from: &str, to: &str) -> Result<()> {
        // Extract value from source
        let value = self.extract_value(schema, from)?;

        // Add to destination (without removing source)
        self.apply_add(schema, to, &value)?;

        Ok(())
    }

    /// Apply a test operation
    fn apply_test(&self, schema: &SchemaDefinition, path: &str, expected: &Value) -> Result<()> {
        let actual = self.extract_value(schema, path)?;

        if actual == *expected {
            Ok(())
        } else {
            Err(LinkMLError::service(format!(
                "Test failed at path '{path}': expected {expected:?}, got {actual:?}"
            )))
        }
    }

    /// Extract a value from the schema at the given path
    fn extract_value(&self, schema: &SchemaDefinition, path: &str) -> Result<Value> {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

        match parts.as_slice() {
            ["classes", class_name] => schema
                .classes
                .get(*class_name)
                .ok_or_else(|| LinkMLError::service(format!("Class '{class_name}' not found")))
                .and_then(|c| {
                    serde_json::to_value(c).map_err(|e| {
                        LinkMLError::service(format!(
                            "Failed to serialize class '{class_name}': {e}"
                        ))
                    })
                }),
            ["slots", slot_name] => schema
                .slots
                .get(*slot_name)
                .ok_or_else(|| LinkMLError::service(format!("Slot '{slot_name}' not found")))
                .and_then(|s| {
                    serde_json::to_value(s).map_err(|e| {
                        LinkMLError::service(format!("Failed to serialize slot '{slot_name}': {e}"))
                    })
                }),
            _ => Err(LinkMLError::service(format!("Unsupported path: {path}"))),
        }
    }

    /// Validate a schema
    fn validate_schema(&self, schema: &SchemaDefinition) -> Result<()> {
        // Basic validation - could be expanded
        if schema.name.is_empty() {
            return Err(LinkMLError::schema_validation("Schema name is empty"));
        }
        Ok(())
    }
}

/// Create a patch from a diff result
#[must_use]
pub fn create_patch_from_diff(diff: &DiffResult) -> SchemaPatch {
    let mut operations = Vec::new();

    // Add operations for new classes
    for class_name in &diff.added_classes {
        operations.push(PatchOperation::Add {
            path: format!("/classes/{class_name}"),
            value: Value::Object(serde_json::Map::new()),
        });
    }

    // Remove operations for deleted classes
    for class_name in &diff.removed_classes {
        operations.push(PatchOperation::Remove {
            path: format!("/classes/{class_name}"),
        });
    }

    // Modify operations for changed classes
    for class_diff in &diff.modified_classes {
        // Add new slots
        for slot in &class_diff.added_slots {
            operations.push(PatchOperation::Add {
                path: format!("/classes/{}/slots/{}", class_diff.name, slot),
                value: Value::String(slot.clone()),
            });
        }

        // Remove deleted slots
        for slot in &class_diff.removed_slots {
            operations.push(PatchOperation::Remove {
                path: format!("/classes/{}/slots/{}", class_diff.name, slot),
            });
        }

        // Replace changed attributes
        for (attr, change) in &class_diff.changed_attributes {
            if let Some(new_value) = &change.new_value {
                operations.push(PatchOperation::Replace {
                    path: format!("/classes/{}/{}", class_diff.name, attr),
                    value: new_value.clone(),
                });
            }
        }
    }

    // Similar for slots, types, enums...
    for slot_name in &diff.added_slots {
        operations.push(PatchOperation::Add {
            path: format!("/slots/{slot_name}"),
            value: Value::Object(serde_json::Map::new()),
        });
    }

    for slot_name in &diff.removed_slots {
        operations.push(PatchOperation::Remove {
            path: format!("/slots/{slot_name}"),
        });
    }

    SchemaPatch {
        operations,
        description: Some("Auto-generated patch from diff".to_string()),
        from_version: None,
        to_version: None,
        breaking: !diff.breaking_changes.is_empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{SchemaDefinition, SlotDefinition};

    #[test]
    fn test_add_class() {
        let mut schema = SchemaDefinition::default();
        schema.name = "test".to_string();

        let patcher = SchemaPatcher::new(PatchOptions::default());

        let patch = SchemaPatch {
            operations: vec![PatchOperation::Add {
                path: "/classes/Person".to_string(),
                value: serde_json::json!({
                    "name": "Person",
                    "description": "A person",
                    "slots": ["name", "age"]
                }),
            }],
            description: None,
            from_version: None,
            to_version: None,
            breaking: false,
        };

        let result = patcher
            .apply_patch(schema, &patch)
            .expect("Should apply patch");
        assert_eq!(result.applied_operations.len(), 1);
        assert!(result.schema.classes.contains_key("Person"));
    }

    #[test]
    fn test_remove_slot() {
        let mut schema = SchemaDefinition::default();
        schema.name = "test".to_string();

        let mut slot = SlotDefinition::default();
        slot.name = "old_slot".to_string();
        schema.slots.insert("old_slot".to_string(), slot);

        let patch = SchemaPatch {
            operations: vec![PatchOperation::Remove {
                path: "/slots/old_slot".to_string(),
            }],
            description: None,
            from_version: None,
            to_version: None,
            breaking: true,
        };

        let mut options = PatchOptions::default();
        options.allow_breaking = true;
        let patcher = SchemaPatcher::new(options);

        let result = patcher
            .apply_patch(schema, &patch)
            .expect("Should apply patch");
        assert!(!result.schema.slots.contains_key("old_slot"));
    }
}
