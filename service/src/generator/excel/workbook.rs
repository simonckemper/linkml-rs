use super::super::traits::{Generator, GeneratorError, GeneratorResult};
use super::generator::ExcelGenerator;
use async_trait::async_trait;
use linkml_core::prelude::*;
use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, Workbook};

impl ExcelGenerator {
    /// Generate Excel file and save to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if workbook generation fails or the file cannot be written.
    pub fn generate_file(&self, schema: &SchemaDefinition, path: &str) -> GeneratorResult<()> {
        let content = self.generate_workbook(schema)?;
        std::fs::write(path, content)
            .map_err(|e| GeneratorError::Generation(format!("Failed to write file {path}: {e}")))?;
        Ok(())
    }

    /// Generate the Excel workbook as a byte buffer.
    fn generate_workbook(&self, schema: &SchemaDefinition) -> GeneratorResult<Vec<u8>> {
        let mut workbook = Workbook::new();

        // Precompute formats shared across sheets.
        let header_format = Format::new()
            .set_bold()
            .set_background_color(Color::Gray)
            .set_font_color(Color::White)
            .set_align(FormatAlign::Center)
            .set_border(FormatBorder::Thin);

        let required_format = Format::new()
            .set_background_color(Color::RGB(0x00FF_EBCD))
            .set_border(FormatBorder::Thin);

        let optional_format = Format::new().set_border(FormatBorder::Thin);

        let type_format = Format::new()
            .set_font_color(Color::Blue)
            .set_italic()
            .set_border(FormatBorder::Thin);

        if self.include_summary() {
            self.generate_summary_sheet(&mut workbook, schema, &header_format)?;
        }

        for (class_name, class_def) in &schema.classes {
            if class_def.abstract_.unwrap_or(false) {
                continue;
            }

            self.generate_class_sheet(
                &mut workbook,
                class_name,
                class_def,
                schema,
                &header_format,
                &required_format,
                &optional_format,
                &type_format,
            )?;
        }

        if !schema.enums.is_empty() {
            self.generate_enums_sheet(&mut workbook, schema, &header_format)?;
        }

        if self.add_validation() {
            self.generate_validation_sheet(&mut workbook, schema, &header_format)?;
        }

        workbook
            .save_to_buffer()
            .map_err(|e| GeneratorError::Generation(format!("Failed to save workbook: {e}")))
    }
}

impl Default for ExcelGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Generator for ExcelGenerator {
    fn name(&self) -> &'static str {
        "excel"
    }

    fn description(&self) -> &'static str {
        "Generate Excel workbooks from LinkML schemas"
    }

    fn file_extensions(&self) -> Vec<&'static str> {
        vec![".xlsx"]
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> std::result::Result<(), LinkMLError> {
        if schema.name.is_empty() {
            return Err(LinkMLError::data_validation(
                "Schema must have a name for Excel generation",
            ));
        }

        let concrete_classes = schema
            .classes
            .iter()
            .filter(|(_, c)| !c.abstract_.unwrap_or(false))
            .count();

        if concrete_classes == 0 {
            return Err(LinkMLError::data_validation(
                "Schema must have at least one concrete (non-abstract) class for Excel generation",
            ));
        }

        for (class_name, _) in &schema.classes {
            if class_name.len() > 31 {
                return Err(LinkMLError::data_validation(format!(
                    "Class name '{class_name}' exceeds Excel's 31 character worksheet name limit"
                )));
            }
        }

        Ok(())
    }

    fn generate(&self, schema: &SchemaDefinition) -> std::result::Result<String, LinkMLError> {
        use base64::Engine;

        let content = self
            .generate_workbook(schema)
            .map_err(|e| LinkMLError::service(format!("Excel generation error: {e}")))?;

        Ok(base64::engine::general_purpose::STANDARD.encode(&content))
    }

    fn get_file_extension(&self) -> &'static str {
        "xlsx"
    }

    fn get_default_filename(&self) -> &'static str {
        "schema"
    }
}

#[cfg(test)]
mod tests {
    use super::ExcelGenerator;
    use crate::generator::traits::Generator;
    use indexmap::IndexMap;
    use linkml_core::types::{ClassDefinition, EnumDefinition, SchemaDefinition, SlotDefinition};

    fn create_test_schema() -> SchemaDefinition {
        let person_class = ClassDefinition {
            slots: vec!["name".to_string(), "age".to_string(), "status".to_string()],
            ..Default::default()
        };

        let mut classes = IndexMap::new();
        classes.insert("Person".to_string(), person_class);

        let name_slot = SlotDefinition {
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        };

        let age_slot = SlotDefinition {
            range: Some("integer".to_string()),
            minimum_value: Some(serde_json::json!(0)),
            maximum_value: Some(serde_json::json!(150)),
            ..Default::default()
        };

        let status_slot = SlotDefinition {
            range: Some("Status".to_string()),
            ..Default::default()
        };

        let mut slots = IndexMap::new();
        slots.insert("name".to_string(), name_slot);
        slots.insert("age".to_string(), age_slot);
        slots.insert("status".to_string(), status_slot);

        let status_enum = EnumDefinition {
            permissible_values: vec![
                linkml_core::types::PermissibleValue::Simple("ACTIVE".to_string()),
                linkml_core::types::PermissibleValue::Simple("INACTIVE".to_string()),
            ],
            ..Default::default()
        };

        let mut enums = IndexMap::new();
        enums.insert("Status".to_string(), status_enum);

        SchemaDefinition {
            name: "TestSchema".to_string(),
            description: Some("A test schema for Excel generation".to_string()),
            classes,
            slots,
            enums,
            ..Default::default()
        }
    }

    #[test]
    fn test_excel_generation() -> anyhow::Result<()> {
        let schema = create_test_schema();
        let generator = ExcelGenerator::new();

        let result = generator
            .generate(&schema)
            .expect("should generate Excel: {}");
        assert!(!result.is_empty());
        Ok(())
    }

    #[test]
    fn test_sheet_name_sanitization() -> std::result::Result<(), Box<dyn std::error::Error>> {
        assert_eq!(ExcelGenerator::sanitize_sheet_name("Simple"), "Simple");
        assert_eq!(
            ExcelGenerator::sanitize_sheet_name("With/Slash"),
            "WithSlash"
        );
        assert_eq!(
            ExcelGenerator::sanitize_sheet_name("With?Question"),
            "WithQuestion"
        );
        assert_eq!(
            ExcelGenerator::sanitize_sheet_name(&"A".repeat(40)),
            "A".repeat(31)
        );
        Ok(())
    }
}
