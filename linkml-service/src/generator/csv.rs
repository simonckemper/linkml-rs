//! CSV/TSV generator for LinkML schemas
//!
//! This generator creates CSV or TSV files from LinkML schemas,
//! flattening the class hierarchy into tabular format.

use super::traits::{Generator, GeneratorOptions, GeneratorResult, GeneratedOutput};
use linkml_core::prelude::*;
use async_trait::async_trait;
use std::collections::BTreeMap;

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
        }
    }

    /// Create a TSV generator
    #[must_use]
    pub fn tsv() -> Self {
        Self {
            delimiter: '\t',
            include_headers: true,
            quote_char: '"',
            use_tabs: true,
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
    fn generate_class_csv(&self, 
        class_name: &str, 
        class_def: &ClassDefinition,
        schema: &SchemaDefinition
    ) -> GeneratorResult<String> {
        let mut output = String::new();
        
        // Collect all slots for this class
        let slots = self.collect_class_slots(class_name, class_def, schema)?;
        
        if slots.is_empty() {
            return Ok(output);
        }
        
        // Generate header
        if self.include_headers {
            let header: Vec<String> = slots.iter()
                .map(|(name, _)| self.escape_field(name))
                .collect();
            output.push_str(&header.join(&self.delimiter.to_string()));
            output.push('\n');
        }
        
        // Generate example row with type information
        let type_row: Vec<String> = slots.iter()
            .map(|(_, slot)| {
                let range = slot.range.as_deref().unwrap_or("string");
                self.escape_field(&format!("<{}>", range))
            })
            .collect();
        output.push_str(&type_row.join(&self.delimiter.to_string()));
        output.push('\n');
        
        // Generate example row with constraints
        let constraint_row: Vec<String> = slots.iter()
            .map(|(name, slot)| {
                let mut constraints = Vec::new();
                
                if slot.required.unwrap_or(false) {
                    constraints.push("required");
                }
                
                if let Some(pattern) = &slot.pattern {
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
    fn collect_class_slots(&self, 
        class_name: &str,
        class_def: &ClassDefinition, 
        schema: &SchemaDefinition
    ) -> GeneratorResult<Vec<(String, SlotDefinition)>> {
        let mut slots = BTreeMap::new();
        
        // Get inherited slots
        if let Some(parent) = &class_def.is_a {
            if let Some(parent_class) = schema.classes.get(parent) {
                let parent_slots = self.collect_class_slots(parent, parent_class, schema)?;
                for (name, slot) in parent_slots {
                    slots.insert(name, slot);
                }
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
        if value.contains(self.delimiter) || 
           value.contains(self.quote_char) || 
           value.contains('\n') || 
           value.contains('\r') {
            // Escape quotes by doubling them
            let escaped = value.replace(self.quote_char, &format!("{}{}", self.quote_char, self.quote_char));
            format!("{}{}{}", self.quote_char, escaped, self.quote_char)
        } else {
            value.to_string()
        }
    }

    /// Generate a sample data row
    fn generate_sample_row(&self, slots: &[(String, SlotDefinition)]) -> String {
        let values: Vec<String> = slots.iter()
            .map(|(name, slot)| {
                let value = match slot.range.as_deref() {
                    Some("string") => format!("Sample {}", name),
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
        
        format!("{}\n", values.join(&self.delimiter.to_string()))
    }

    /// Generate schema summary CSV
    fn generate_schema_summary(&self, schema: &SchemaDefinition) -> String {
        let mut output = String::new();
        
        // Header
        if self.include_headers {
            output.push_str(&format!("Type{}Name{}Description{}Count\n",
                self.delimiter, self.delimiter, self.delimiter));
        }
        
        // Classes
        output.push_str(&format!("Class{}{}{}Schema classes{}{}\n",
            self.delimiter, self.delimiter, self.delimiter, self.delimiter, schema.classes.len()));
        
        // Slots
        output.push_str(&format!("Slot{}{}{}Schema slots{}{}\n",
            self.delimiter, self.delimiter, self.delimiter, self.delimiter, schema.slots.len()));
        
        // Enums
        output.push_str(&format!("Enum{}{}{}Schema enumerations{}{}\n",
            self.delimiter, self.delimiter, self.delimiter, self.delimiter, schema.enums.len()));
        
        // Types
        output.push_str(&format!("Type{}{}{}Schema types{}{}\n",
            self.delimiter, self.delimiter, self.delimiter, self.delimiter, schema.types.len()));
        
        output
    }
}

impl Default for CsvGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Generator for CsvGenerator {
    fn name(&self) -> &str {
        if self.use_tabs {
            "tsv"
        } else {
            "csv"
        }
    }

    fn description(&self) -> &'static str {
        if self.use_tabs {
            "Generate TSV (Tab-Separated Values) files from LinkML schemas"
        } else {
            "Generate CSV (Comma-Separated Values) files from LinkML schemas"
        }
    }

    fn file_extensions(&self) -> Vec<&str> {
        if self.use_tabs {
            vec![".tsv", ".txt"]
        } else {
            vec![".csv"]
        }
    }

    async fn generate(
        &self,
        schema: &SchemaDefinition,
        _options: &GeneratorOptions,
    ) -> GeneratorResult<Vec<GeneratedOutput>> {
        let mut outputs = Vec::new();
        
        // Generate summary file
        outputs.push(GeneratedOutput {
            filename: format!("{}_summary.{}", 
                if schema.name.is_empty() { "schema" } else { &schema.name },
                if self.use_tabs { "tsv" } else { "csv" }
            ),
            content: self.generate_schema_summary(schema),
            metadata: std::collections::HashMap::new(),
        });
        
        // Generate file for each non-abstract class
        for (class_name, class_def) in &schema.classes {
            if class_def.abstract_.unwrap_or(false) {
                continue;
            }
            
            let content = self.generate_class_csv(class_name, class_def, schema)?;
            
            if !content.is_empty() {
                outputs.push(GeneratedOutput {
                    filename: format!("{}.{}", 
                        class_name.to_lowercase(),
                        if self.use_tabs { "tsv" } else { "csv" }
                    ),
                    content,
                    metadata: std::collections::HashMap::new(),
                });
            }
        }
        
        Ok(outputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_schema() -> SchemaDefinition {
        let mut schema = SchemaDefinition::default();
        schema.name = Some("TestSchema".to_string());
        
        // Base class
        let mut entity_class = ClassDefinition::default();
        entity_class.abstract_ = Some(true);
        entity_class.slots = vec!["id".to_string()];
        schema.classes.insert("Entity".to_string(), entity_class);
        
        // Person class
        let mut person_class = ClassDefinition::default();
        person_class.is_a = Some("Entity".to_string());
        person_class.slots = vec!["name".to_string(), "age".to_string()];
        schema.classes.insert("Person".to_string(), person_class);
        
        // Slots
        let mut id_slot = SlotDefinition::default();
        id_slot.range = Some("string".to_string());
        id_slot.required = Some(true);
        schema.slots.insert("id".to_string(), id_slot);
        
        let mut name_slot = SlotDefinition::default();
        name_slot.range = Some("string".to_string());
        name_slot.required = Some(true);
        schema.slots.insert("name".to_string(), name_slot);
        
        let mut age_slot = SlotDefinition::default();
        age_slot.range = Some("integer".to_string());
        schema.slots.insert("age".to_string(), age_slot);
        
        schema
    }

    #[tokio::test]
    async fn test_csv_generation() {
        let schema = create_test_schema();
        let generator = CsvGenerator::new();
        let options = GeneratorOptions::default();
        
        let result = generator.generate(&schema, &options).await.expect("should generate CSV");
        
        // Should generate summary + person.csv (entity is abstract)
        assert_eq!(result.len(), 2);
        
        // Check summary file
        let summary = result.iter().find(|o| o.filename.contains("summary")).expect("should have summary file");
        assert!(summary.content.contains("Class"));
        assert!(summary.content.contains("2")); // 2 classes
        
        // Check person file
        let person = result.iter().find(|o| o.filename == "person.csv").expect("should have person.csv");
        assert!(person.content.contains("id,name,age"));
        assert!(person.content.contains("<string>,<string>,<integer>"));
    }

    #[tokio::test]
    async fn test_tsv_generation() {
        let schema = create_test_schema();
        let generator = CsvGenerator::tsv();
        let options = GeneratorOptions::default();
        
        let result = generator.generate(&schema, &options).await.expect("should generate TSV");
        
        let person = result.iter().find(|o| o.filename == "person.tsv").expect("should have person.tsv");
        assert!(person.content.contains("id\tname\tage"));
    }

    #[test]
    fn test_field_escaping() {
        let generator = CsvGenerator::new();
        
        // Simple field
        assert_eq!(generator.escape_field("simple"), "simple");
        
        // Field with comma
        assert_eq!(generator.escape_field("value,with,comma"), "\"value,with,comma\"");
        
        // Field with quotes
        assert_eq!(generator.escape_field("value\"with\"quotes"), "\"value\"\"with\"\"quotes\"");
        
        // Field with newline
        assert_eq!(generator.escape_field("value\nwith\nnewline"), "\"value\nwith\nnewline\"");
    }
}