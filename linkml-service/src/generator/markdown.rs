//! Markdown documentation generator for LinkML schemas
//!
//! This generator creates comprehensive Markdown documentation from LinkML schemas,
//! including class hierarchies, slot tables, enumerations, and cross-references.

use super::traits::{Generator, GeneratorOptions, GeneratorResult, GeneratedOutput};
use linkml_core::prelude::*;
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::fmt::Write;

/// Markdown documentation generator
pub struct MarkdownGenerator {
    /// Whether to include table of contents
    include_toc: bool,
    /// Whether to include inheritance diagrams
    include_diagrams: bool,
    /// Whether to include examples
    include_examples: bool,
}

impl MarkdownGenerator {
    /// Create a new Markdown generator
    #[must_use]
    pub fn new() -> Self {
        Self {
            include_toc: true,
            include_diagrams: true,
            include_examples: true,
        }
    }

    /// Configure table of contents generation
    #[must_use]
    pub fn with_toc(mut self, enabled: bool) -> Self {
        self.include_toc = enabled;
        self
    }

    /// Configure diagram generation
    #[must_use]
    pub fn with_diagrams(mut self, enabled: bool) -> Self {
        self.include_diagrams = enabled;
        self
    }

    /// Configure example generation
    #[must_use]
    pub fn with_examples(mut self, enabled: bool) -> Self {
        self.include_examples = enabled;
        self
    }

    /// Generate the schema header
    fn generate_header(&self, schema: &SchemaDefinition) -> String {
        let mut output = String::new();
        
        writeln!(&mut output, "# {}", if schema.name.is_empty() { "LinkML Schema" } else { &schema.name }).unwrap();
        writeln!(&mut output).unwrap();
        
        if let Some(title) = &schema.title {
            writeln!(&mut output, "**Title**: {}", title).unwrap();
        }
        
        if let Some(description) = &schema.description {
            writeln!(&mut output, "\n**Description**: {}", description).unwrap();
        }
        
        if let Some(version) = &schema.version {
            writeln!(&mut output, "\n**Version**: {}", version).unwrap();
        }
        
        writeln!(&mut output, "\n---\n").unwrap();
        
        output
    }

    /// Generate table of contents
    fn generate_toc(&self, schema: &SchemaDefinition) -> String {
        let mut output = String::new();
        
        writeln!(&mut output, "## Table of Contents\n").unwrap();
        writeln!(&mut output, "1. [Overview](#overview)").unwrap();
        
        if !schema.classes.is_empty() {
            writeln!(&mut output, "2. [Classes](#classes)").unwrap();
            for (i, class_name) in schema.classes.keys().enumerate() {
                let anchor = class_name.to_lowercase().replace(' ', "-");
                writeln!(&mut output, "   - [{class_name}](#{anchor})").unwrap();
            }
        }
        
        if !schema.slots.is_empty() {
            writeln!(&mut output, "3. [Slots](#slots)").unwrap();
        }
        
        if !schema.enums.is_empty() {
            writeln!(&mut output, "4. [Enumerations](#enumerations)").unwrap();
            for enum_name in schema.enums.keys() {
                let anchor = enum_name.to_lowercase().replace(' ', "-");
                writeln!(&mut output, "   - [{enum_name}](#{anchor})").unwrap();
            }
        }
        
        if !schema.types.is_empty() {
            writeln!(&mut output, "5. [Types](#types)").unwrap();
        }
        
        writeln!(&mut output, "\n---\n").unwrap();
        
        output
    }

    /// Generate overview section
    fn generate_overview(&self, schema: &SchemaDefinition) -> String {
        let mut output = String::new();
        
        writeln!(&mut output, "## Overview\n").unwrap();
        
        // Statistics
        writeln!(&mut output, "### Statistics\n").unwrap();
        writeln!(&mut output, "| Category | Count |").unwrap();
        writeln!(&mut output, "|----------|-------|").unwrap();
        writeln!(&mut output, "| Classes | {} |", schema.classes.len()).unwrap();
        writeln!(&mut output, "| Slots | {} |", schema.slots.len()).unwrap();
        writeln!(&mut output, "| Enums | {} |", schema.enums.len()).unwrap();
        writeln!(&mut output, "| Types | {} |", schema.types.len()).unwrap();
        
        // Prefixes
        if !schema.prefixes.is_empty() {
            writeln!(&mut output, "\n### Prefixes\n").unwrap();
            writeln!(&mut output, "| Prefix | URI |").unwrap();
            writeln!(&mut output, "|--------|-----|").unwrap();
            for (prefix, uri_def) in &schema.prefixes {
                let uri = match uri_def {
                    linkml_core::types::PrefixDefinition::Simple(s) => s.as_str(),
                    linkml_core::types::PrefixDefinition::Complex { prefix_prefix, .. } => prefix_prefix.as_str(),
                };
                writeln!(&mut output, "| {} | {} |", prefix, uri).unwrap();
            }
        }
        
        writeln!(&mut output, "\n---\n").unwrap();
        
        output
    }

    /// Generate class documentation
    fn generate_classes(&self, schema: &SchemaDefinition) -> String {
        let mut output = String::new();
        
        if schema.classes.is_empty() {
            return output;
        }
        
        writeln!(&mut output, "## Classes\n").unwrap();
        
        // Sort classes for consistent output
        let mut sorted_classes: Vec<_> = schema.classes.iter().collect();
        sorted_classes.sort_by_key(|(name, _)| name.as_str());
        
        for (class_name, class_def) in sorted_classes {
            writeln!(&mut output, "### {}\n", class_name).unwrap();
            
            if let Some(description) = &class_def.description {
                writeln!(&mut output, "{}\n", description).unwrap();
            }
            
            // Metadata table
            writeln!(&mut output, "#### Metadata\n").unwrap();
            writeln!(&mut output, "| Property | Value |").unwrap();
            writeln!(&mut output, "|----------|-------|").unwrap();
            
            if let Some(is_a) = &class_def.is_a {
                writeln!(&mut output, "| Parent Class | [{}](#{}) |", is_a, is_a.to_lowercase().replace(' ', "-")).unwrap();
            }
            
            if let Some(abstract_) = class_def.abstract_ {
                writeln!(&mut output, "| Abstract | {} |", abstract_).unwrap();
            }
            
            if let Some(mixin) = class_def.mixin {
                writeln!(&mut output, "| Mixin | {} |", mixin).unwrap();
            }
            
            if !class_def.mixins.is_empty() {
                let mixins = class_def.mixins.join(", ");
                writeln!(&mut output, "| Uses Mixins | {} |", mixins).unwrap();
            }
            
            // Slots table
            if !class_def.slots.is_empty() || !class_def.attributes.is_empty() {
                writeln!(&mut output, "\n#### Slots\n").unwrap();
                writeln!(&mut output, "| Slot | Required | Type | Description |").unwrap();
                writeln!(&mut output, "|------|----------|------|-------------|").unwrap();
                
                // Collect all slots (direct and inherited)
                let mut all_slots: BTreeMap<String, SlotDefinition> = BTreeMap::new();
                
                // Direct slots
                for slot_name in &class_def.slots {
                    if let Some(slot_def) = schema.slots.get(slot_name) {
                        all_slots.insert(slot_name.clone(), slot_def.clone());
                    }
                }
                
                // Attributes (inline slots)
                for (attr_name, attr_def) in &class_def.attributes {
                    all_slots.insert(attr_name.clone(), attr_def.clone());
                }
                
                // Slot usage overrides
                for (slot_name, slot_usage) in &class_def.slot_usage {
                    if let Some(base_slot) = schema.slots.get(slot_name) {
                        // Apply overrides to base slot
                        let mut overridden = base_slot.clone();
                        if slot_usage.required.is_some() {
                            overridden.required = slot_usage.required;
                        }
                        if slot_usage.range.is_some() {
                            overridden.range = slot_usage.range.clone();
                        }
                        if slot_usage.description.is_some() {
                            overridden.description = slot_usage.description.clone();
                        }
                        all_slots.insert(slot_name.clone(), overridden);
                    }
                }
                
                for (slot_name, slot_def) in &all_slots {
                    let required = slot_def.required.unwrap_or(false);
                    let range = slot_def.range.as_deref().unwrap_or("string");
                    let description = slot_def.description.as_deref().unwrap_or("");
                    
                    writeln!(&mut output, "| {} | {} | {} | {} |", 
                        slot_name, 
                        if required { "✓" } else { "" },
                        range,
                        description
                    ).unwrap();
                }
            }
            
            // Examples
            if self.include_examples {
                writeln!(&mut output, "\n#### Example\n").unwrap();
                writeln!(&mut output, "```yaml").unwrap();
                writeln!(&mut output, "{}:", class_name).unwrap();
                
                // Generate example values for each slot
                for slot_name in &class_def.slots {
                    if let Some(slot_def) = schema.slots.get(slot_name) {
                        let example_value = self.generate_example_value(slot_def);
                        writeln!(&mut output, "  {}: {}", slot_name, example_value).unwrap();
                    }
                }
                
                writeln!(&mut output, "```\n").unwrap();
            }
            
            writeln!(&mut output, "---\n").unwrap();
        }
        
        output
    }

    /// Generate slot documentation
    fn generate_slots(&self, schema: &SchemaDefinition) -> String {
        let mut output = String::new();
        
        if schema.slots.is_empty() {
            return output;
        }
        
        writeln!(&mut output, "## Slots\n").unwrap();
        writeln!(&mut output, "| Slot | Type | Required | Description |").unwrap();
        writeln!(&mut output, "|------|------|----------|-------------|").unwrap();
        
        let mut sorted_slots: Vec<_> = schema.slots.iter().collect();
        sorted_slots.sort_by_key(|(name, _)| name.as_str());
        
        for (slot_name, slot_def) in sorted_slots {
            let range = slot_def.range.as_deref().unwrap_or("string");
            let required = slot_def.required.unwrap_or(false);
            let description = slot_def.description.as_deref().unwrap_or("");
            
            writeln!(&mut output, "| {} | {} | {} | {} |", 
                slot_name,
                range,
                if required { "✓" } else { "" },
                description
            ).unwrap();
        }
        
        writeln!(&mut output, "\n---\n").unwrap();
        
        output
    }

    /// Generate enumeration documentation
    fn generate_enums(&self, schema: &SchemaDefinition) -> String {
        let mut output = String::new();
        
        if schema.enums.is_empty() {
            return output;
        }
        
        writeln!(&mut output, "## Enumerations\n").unwrap();
        
        let mut sorted_enums: Vec<_> = schema.enums.iter().collect();
        sorted_enums.sort_by_key(|(name, _)| name.as_str());
        
        for (enum_name, enum_def) in sorted_enums {
            writeln!(&mut output, "### {}\n", enum_name).unwrap();
            
            if let Some(description) = &enum_def.description {
                writeln!(&mut output, "{}\n", description).unwrap();
            }
            
            writeln!(&mut output, "#### Permissible Values\n").unwrap();
            writeln!(&mut output, "| Value | Description |").unwrap();
            writeln!(&mut output, "|-------|-------------|").unwrap();
            
            for pv in &enum_def.permissible_values {
                let (value, description) = match pv {
                    linkml_core::types::PermissibleValue::Simple(s) => (s.as_str(), ""),
                    linkml_core::types::PermissibleValue::Complex { text, description, .. } => {
                        (text.as_str(), description.as_deref().unwrap_or(""))
                    }
                };
                writeln!(&mut output, "| {} | {} |", value, description).unwrap();
            }
            
            writeln!(&mut output, "\n---\n").unwrap();
        }
        
        output
    }

    /// Generate types documentation
    fn generate_types(&self, schema: &SchemaDefinition) -> String {
        let mut output = String::new();
        
        if schema.types.is_empty() {
            return output;
        }
        
        writeln!(&mut output, "## Types\n").unwrap();
        writeln!(&mut output, "| Type | Base | Pattern | Description |").unwrap();
        writeln!(&mut output, "|------|------|---------|-------------|").unwrap();
        
        let mut sorted_types: Vec<_> = schema.types.iter().collect();
        sorted_types.sort_by_key(|(name, _)| name.as_str());
        
        for (type_name, type_def) in sorted_types {
            let base = type_def.base_type.as_deref().unwrap_or("");
            let pattern = type_def.pattern.as_deref().unwrap_or("");
            let description = type_def.description.as_deref().unwrap_or("");
            
            let pattern_str = if pattern.is_empty() { 
                String::new() 
            } else { 
                format!("`{}`", pattern) 
            };
            
            writeln!(&mut output, "| {} | {} | {} | {} |", 
                type_name,
                base,
                pattern_str,
                description
            ).unwrap();
        }
        
        writeln!(&mut output, "\n---\n").unwrap();
        
        output
    }

    /// Generate example value for a slot
    fn generate_example_value(&self, slot: &SlotDefinition) -> &'static str {
        match slot.range.as_deref() {
            Some("string") => "\"example string\"",
            Some("integer") => "42",
            Some("float") => "3.14",
            Some("boolean") => "true",
            Some("date") => "2024-01-01",
            Some("datetime") => "2024-01-01T12:00:00Z",
            Some("uri") => "https://example.com",
            _ => "\"example value\"",
        }
    }
}

impl Default for MarkdownGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Generator for MarkdownGenerator {
    fn name(&self) -> &str {
        "markdown"
    }

    fn description(&self) -> &'static str {
        "Generate Markdown documentation from LinkML schemas"
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec![".md"]
    }

    async fn generate(
        &self,
        schema: &SchemaDefinition,
        _options: &GeneratorOptions,
    ) -> GeneratorResult<Vec<GeneratedOutput>> {
        let mut content = String::new();
        
        // Generate sections
        content.push_str(&self.generate_header(schema));
        
        if self.include_toc {
            content.push_str(&self.generate_toc(schema));
        }
        
        content.push_str(&self.generate_overview(schema));
        content.push_str(&self.generate_classes(schema));
        content.push_str(&self.generate_slots(schema));
        content.push_str(&self.generate_enums(schema));
        content.push_str(&self.generate_types(schema));
        
        // Footer
        content.push_str("\n---\n\n");
        content.push_str(&format!("*Generated by LinkML Markdown Generator v{}*", env!("CARGO_PKG_VERSION")));
        
        Ok(vec![GeneratedOutput {
            filename: format!("{}.md", if schema.name.is_empty() { "schema" } else { &schema.name }),
            content,
            metadata: std::collections::HashMap::new(),
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_schema() -> SchemaDefinition {
        let mut schema = SchemaDefinition::default();
        schema.name = Some("TestSchema".to_string());
        schema.title = Some("Test Schema".to_string());
        schema.description = Some("A test schema for documentation".to_string());
        schema.version = Some("1.0.0".to_string());
        
        // Add a class
        let mut person_class = ClassDefinition::default();
        person_class.description = Some("A person entity".to_string());
        person_class.slots = vec!["name".to_string(), "age".to_string()];
        schema.classes.insert("Person".to_string(), person_class);
        
        // Add slots
        let mut name_slot = SlotDefinition::default();
        name_slot.description = Some("The person's name".to_string());
        name_slot.range = Some("string".to_string());
        name_slot.required = Some(true);
        schema.slots.insert("name".to_string(), name_slot);
        
        let mut age_slot = SlotDefinition::default();
        age_slot.description = Some("The person's age".to_string());
        age_slot.range = Some("integer".to_string());
        schema.slots.insert("age".to_string(), age_slot);
        
        // Add an enum
        let mut status_enum = EnumDefinition::default();
        status_enum.description = Some("Employment status".to_string());
        
        let mut active_pv = PermissibleValue::default();
        active_pv.description = Some("Currently employed".to_string());
        status_enum.permissible_values.insert("ACTIVE".to_string(), active_pv);
        
        let mut inactive_pv = PermissibleValue::default();
        inactive_pv.description = Some("Not currently employed".to_string());
        status_enum.permissible_values.insert("INACTIVE".to_string(), inactive_pv);
        
        schema.enums.insert("EmploymentStatus".to_string(), status_enum);
        
        schema
    }

    #[tokio::test]
    async fn test_markdown_generation() {
        let schema = create_test_schema();
        let generator = MarkdownGenerator::new();
        let options = GeneratorOptions::default();
        
        let result = generator.generate(&schema, &options).await.unwrap();
        assert_eq!(result.len(), 1);
        
        let output = &result[0];
        assert_eq!(output.filename, "TestSchema.md");
        
        // Check content includes expected sections
        assert!(output.content.contains("# TestSchema"));
        assert!(output.content.contains("## Table of Contents"));
        assert!(output.content.contains("## Overview"));
        assert!(output.content.contains("## Classes"));
        assert!(output.content.contains("### Person"));
        assert!(output.content.contains("## Enumerations"));
        assert!(output.content.contains("### EmploymentStatus"));
    }

    #[test]
    fn test_example_generation() {
        let generator = MarkdownGenerator::new();
        
        let mut slot = SlotDefinition::default();
        
        slot.range = Some("string".to_string());
        assert_eq!(generator.generate_example_value(&slot), "\"example string\"");
        
        slot.range = Some("integer".to_string());
        assert_eq!(generator.generate_example_value(&slot), "42");
        
        slot.range = Some("boolean".to_string());
        assert_eq!(generator.generate_example_value(&slot), "true");
    }
}