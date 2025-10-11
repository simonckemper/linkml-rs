//! CSV/TSV generator for `LinkML` schemas
//!
//! This generator creates CSV or TSV files from `LinkML` schemas,
//! flattening the class hierarchy into tabular format.

use super::traits::{Generator, GeneratorResult};
use linkml_core::prelude::*;
use std::collections::BTreeMap;
use std::fmt::Write;

/// CSV/TSV generator
pub struct CsvGenerator {
    /// Delimiter character (default: comma)
    delimiter: char,
    /// Whether to include headers
    include_headers: bool,
    /// Quote character for escaping
    quote_char: char,
    /// Whether to generate TSV instead of CSV
    use_tabs: bool,
    /// Generator options
    options: super::traits::GeneratorOptions,
}

impl CsvGenerator {
    /// Create a new CSV generator
    #[must_use]
    pub fn new() -> Self {
        Self {
            delimiter: ',',
            include_headers: true,
            quote_char: '"',
            use_tabs: false,
            options: super::traits::GeneratorOptions::default(),
        }
    }

    /// Create generator with options
    #[must_use]
    pub fn with_options(options: super::traits::GeneratorOptions) -> Self {
        let mut generator = Self::new();
        generator.options = options;
        generator
    }

    /// Create a TSV generator
    #[must_use]
    pub fn tsv() -> Self {
        Self {
            delimiter: '\t',
            include_headers: true,
            quote_char: '"',
            use_tabs: true,
            options: super::traits::GeneratorOptions::default(),
        }
    }

    /// Set custom delimiter
    #[must_use]
    pub fn with_delimiter(mut self, delimiter: char) -> Self {
        self.delimiter = delimiter;
        self
    }

    /// Configure header generation
    #[must_use]
    pub fn with_headers(mut self, enabled: bool) -> Self {
        self.include_headers = enabled;
        self
    }

    /// Generate CSV for a single class
    fn generate_class_csv(
        &self,
        class_name: &str,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<String> {
        let mut output = String::new();

        // Collect all slots for this class
        let slots = self.collect_class_slots(class_name, class_def, schema)?;

        if slots.is_empty() {
            return Ok(output);
        }

        // Generate header
        if self.include_headers {
            let header: Vec<String> = slots
                .iter()
                .map(|(name, _)| self.escape_field(name))
                .collect();
            output.push_str(&header.join(&self.delimiter.to_string()));
            output.push('\n');
        }

        // Generate example row with type information
        let type_row: Vec<String> = slots
            .iter()
            .map(|(_, slot)| {
                let range = slot.range.as_deref().unwrap_or("string");
                self.escape_field(&format!("<{range}>"))
            })
            .collect();
        output.push_str(&type_row.join(&self.delimiter.to_string()));
        output.push('\n');

        // Generate example row with constraints
        let constraint_row: Vec<String> = slots
            .iter()
            .map(|(_name, slot)| {
                let mut constraints = Vec::new();

                if slot.required.unwrap_or(false) {
                    constraints.push("required");
                }

                if let Some(_pattern) = &slot.pattern {
                    constraints.push("pattern");
                }

                if slot.minimum_value.is_some() || slot.maximum_value.is_some() {
                    constraints.push("range");
                }

                if slot.multivalued.unwrap_or(false) {
                    constraints.push("multivalued");
                }

                if constraints.is_empty() {
                    self.escape_field("")
                } else {
                    self.escape_field(&format!("[{}]", constraints.join(", ")))
                }
            })
            .collect();
        output.push_str(&constraint_row.join(&self.delimiter.to_string()));
        output.push('\n');

        // Generate sample data rows
        output.push_str(&self.generate_sample_row(&slots));

        Ok(output)
    }

    /// Collect all slots for a class including inherited ones
    #[allow(clippy::only_used_in_recursion)]
    fn collect_class_slots(
        &self,
        _class_name: &str,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<Vec<(String, SlotDefinition)>> {
        let mut slots = BTreeMap::new();

        // Get inherited slots
        if let Some(parent) = &class_def.is_a
            && let Some(parent_class) = schema.classes.get(parent)
        {
            let parent_slots = self.collect_class_slots(parent, parent_class, schema)?;
            for (name, slot) in parent_slots {
                slots.insert(name, slot);
            }
        }

        // Get mixin slots
        for mixin in &class_def.mixins {
            if let Some(mixin_class) = schema.classes.get(mixin) {
                let mixin_slots = self.collect_class_slots(mixin, mixin_class, schema)?;
                for (name, slot) in mixin_slots {
                    slots.insert(name, slot);
                }
            }
        }

        // Add direct slots
        for slot_name in &class_def.slots {
            if let Some(slot_def) = schema.slots.get(slot_name) {
                slots.insert(slot_name.clone(), slot_def.clone());
            }
        }

        // Add attributes (inline slots)
        for (attr_name, attr_def) in &class_def.attributes {
            slots.insert(attr_name.clone(), attr_def.clone());
        }

        // Apply slot usage overrides
        for (slot_name, slot_usage) in &class_def.slot_usage {
            if let Some(slot) = slots.get_mut(slot_name) {
                if let Some(required) = slot_usage.required {
                    slot.required = Some(required);
                }
                if let Some(ref range) = slot_usage.range {
                    slot.range = Some(range.clone());
                }
                if let Some(ref description) = slot_usage.description {
                    slot.description = Some(description.clone());
                }
            }
        }

        Ok(slots.into_iter().collect())
    }

    /// Escape a field value for CSV
    fn escape_field(&self, value: &str) -> String {
        if value.contains(self.delimiter)
            || value.contains(self.quote_char)
            || value.contains('\n')
            || value.contains('\r')
        {
            // Escape quotes by doubling them
            let escaped = value.replace(
                self.quote_char,
                &format!("{}{}", self.quote_char, self.quote_char),
            );
            format!("{}{}{}", self.quote_char, escaped, self.quote_char)
        } else {
            value.to_string()
        }
    }

    /// Generate a sample data row
    fn generate_sample_row(&self, slots: &[(String, SlotDefinition)]) -> String {
        let values: Vec<String> = slots
            .iter()
            .map(|(name, slot)| {
                let value = match slot.range.as_deref() {
                    Some("string") => format!("Sample {name}"),
                    Some("integer") => "123".to_string(),
                    Some("float") => "45.67".to_string(),
                    Some("boolean") => "true".to_string(),
                    Some("date") => "2024-01-15".to_string(),
                    Some("datetime") => "2024-01-15T10:30:00Z".to_string(),
                    Some("uri") => "https://example.com/resource".to_string(),
                    _ => "sample_value".to_string(),
                };
                self.escape_field(&value)
            })
            .collect();

        format!(
            "{}
",
            values.join(&self.delimiter.to_string())
        )
    }

    /// Generate schema summary CSV
    fn generate_schema_summary(&self, schema: &SchemaDefinition) -> String {
        let mut output = String::new();

        // Header
        if self.include_headers {
            writeln!(
                output,
                "Type{}Name{}Description{}Count",
                self.delimiter, self.delimiter, self.delimiter
            )
            .expect("LinkML operation should succeed");
        }

        // Classes
        writeln!(
            output,
            "Class{}{}{}Schema classes{}{}",
            self.delimiter,
            self.delimiter,
            self.delimiter,
            self.delimiter,
            schema.classes.len()
        )
        .expect("Writing to string cannot fail");

        // Slots
        writeln!(
            output,
            "Slot{}{}{}Schema slots{}{}",
            self.delimiter,
            self.delimiter,
            self.delimiter,
            self.delimiter,
            schema.slots.len()
        )
        .expect("Writing to string cannot fail");

        // Enums
        writeln!(
            output,
            "Enum{}{}{}Schema enumerations{}{}",
            self.delimiter,
            self.delimiter,
            self.delimiter,
            self.delimiter,
            schema.enums.len()
        )
        .expect("Writing to string cannot fail");

        // Types
        writeln!(
            output,
            "Type{}{}{}Schema types{}{}",
            self.delimiter,
            self.delimiter,
            self.delimiter,
            self.delimiter,
            schema.types.len()
        )
        .expect("Writing to string cannot fail");

        output
    }
}

impl Default for CsvGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator for CsvGenerator {
    fn name(&self) -> &str {
        if self.use_tabs { "tsv" } else { "csv" }
    }

    fn description(&self) -> &str {
        if self.use_tabs {
            "Generate TSV (Tab-Separated Values) files from LinkML schemas"
        } else {
            "Generate CSV (Comma-Separated Values) files from LinkML schemas"
        }
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> std::result::Result<(), LinkMLError> {
        // Validate schema has a name
        if schema.name.is_empty() {
            return Err(LinkMLError::data_validation(
                "Schema must have a name for CSV generation",
            ));
        }

        // Validate that we have at least one non-abstract class
        let concrete_classes = schema
            .classes
            .iter()
            .filter(|(_, c)| !c.abstract_.unwrap_or(false))
            .count();

        if concrete_classes == 0 {
            return Err(LinkMLError::data_validation(
                "Schema must have at least one concrete (non-abstract) class for CSV generation",
            ));
        }

        // Validate slot names don't contain special CSV characters
        for (_class_name, class_def) in &schema.classes {
            if !class_def.slots.is_empty() {
                let slots = &class_def.slots;
                for slot_name in slots {
                    if slot_name.contains(',')
                        || slot_name.contains('\n')
                        || slot_name.contains('\r')
                    {
                        return Err(LinkMLError::data_validation(format!(
                            "Slot name '{slot_name}' contains invalid CSV characters"
                        )));
                    }
                }
            }
        }

        Ok(())
    }

    // Sync Generator trait method
    fn generate(&self, schema: &SchemaDefinition) -> std::result::Result<String, LinkMLError> {
        let mut result = String::new();

        // Generate summary
        result.push_str(&self.generate_schema_summary(schema));
        result.push_str(
            "

",
        );

        // Generate content for each class
        for (class_name, class_def) in &schema.classes {
            if class_def.abstract_.unwrap_or(false) {
                continue;
            }

            match self.generate_class_csv(class_name, class_def, schema) {
                Ok(content) => {
                    if !content.is_empty() {
                        writeln!(result, "=== {class_name} ===")
                            .expect("writeln! to String should never fail");
                        result.push_str(&content);
                        result.push_str(
                            "

",
                        );
                    }
                }
                Err(e) => return Err(LinkMLError::service(format!("CSV generation error: {e}"))),
            }
        }

        Ok(result)
    }

    fn get_file_extension(&self) -> &str {
        if self.use_tabs { "tsv" } else { "csv" }
    }

    fn get_default_filename(&self) -> &str {
        if self.use_tabs {
            "schema.tsv"
        } else {
            "schema.csv"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};

    fn create_test_schema() -> SchemaDefinition {
        let mut schema = SchemaDefinition {
            name: "TestSchema".to_string(),
            ..Default::default()
        };

        // Base class
        let entity_class = ClassDefinition {
            abstract_: Some(true),
            slots: vec!["id".to_string()],
            ..Default::default()
        };
        schema.classes.insert("Entity".to_string(), entity_class);

        // Person class
        let person_class = ClassDefinition {
            is_a: Some("Entity".to_string()),
            slots: vec!["name".to_string(), "age".to_string()],
            ..Default::default()
        };
        schema.classes.insert("Person".to_string(), person_class);

        // Slots
        let id_slot = SlotDefinition {
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        };
        schema.slots.insert("id".to_string(), id_slot);

        let name_slot = SlotDefinition {
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        };
        schema.slots.insert("name".to_string(), name_slot);

        let age_slot = SlotDefinition {
            range: Some("integer".to_string()),
            ..Default::default()
        };
        schema.slots.insert("age".to_string(), age_slot);

        schema
    }

    #[test]
    fn test_csv_generation() -> anyhow::Result<()> {
        let schema = create_test_schema();
        let generator = CsvGenerator::new();

        let result = generator
            .generate(&schema)
            .expect("should generate CSV: {}");

        // Should contain summary and person class (entity is abstract)
        assert!(result.contains("Type,Name,Description,Count"));
        assert!(result.contains("Class,,Schema classes,2"));
        assert!(result.contains("=== Person ==="));
        // Slots are in alphabetical order due to BTreeMap
        assert!(result.contains("age,id,name"));
        assert!(result.contains("<integer>,<string>,<string>"));
        Ok(())
    }

    #[test]
    fn test_tsv_generation() -> anyhow::Result<()> {
        let schema = create_test_schema();
        let generator = CsvGenerator::tsv();

        let result = generator
            .generate(&schema)
            .expect("should generate TSV: {}");

        assert!(result.contains("=== Person ==="));
        // Slots are in alphabetical order due to BTreeMap
        assert!(result.contains("age\tid\tname"));
        assert!(result.contains("<integer>\t<string>\t<string>"));
        Ok(())
    }

    #[test]
    fn test_field_escaping() {
        let generator = CsvGenerator::new();

        // Simple field
        assert_eq!(generator.escape_field("simple"), "simple");

        // Field with comma
        assert_eq!(
            generator.escape_field("value,with,comma"),
            "\"value,with,comma\""
        );

        // Field with quotes
        assert_eq!(
            generator.escape_field("value\"with\"quotes"),
            "\"value\"\"with\"\"quotes\""
        );

        // Field with newline
        assert_eq!(
            generator.escape_field(
                "value
with
newline"
            ),
            "\"value
with
newline\""
        );
    }
}
