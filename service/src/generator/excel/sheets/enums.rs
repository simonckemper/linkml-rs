use crate::generator::traits::GeneratorError;
use linkml_core::prelude::SchemaDefinition;
use rust_xlsxwriter::{Format, Worksheet};

use super::super::generator::ExcelGenerator;

impl ExcelGenerator {
    pub(crate) fn generate_enums_sheet(
        &self,
        workbook: &mut rust_xlsxwriter::Workbook,
        schema: &SchemaDefinition,
        header_format: &Format,
    ) -> Result<(), GeneratorError> {
        let mut worksheet = workbook
            .add_worksheet()
            .set_name("Enumerations")
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;

        Self::write_enum_headers(&mut worksheet, header_format)?;
        let mut row = 1;

        for (enum_name, enum_def) in &schema.enums {
            for pv in &enum_def.permissible_values {
                let (value, description) = match pv {
                    linkml_core::types::PermissibleValue::Simple(s) => (s.as_str(), ""),
                    linkml_core::types::PermissibleValue::Complex {
                        text, description, ..
                    } => (text.as_str(), description.as_deref().unwrap_or("")),
                };

                worksheet
                    .write_string(row, 0, enum_name)
                    .map_err(|e| GeneratorError::Generation(e.to_string()))?;
                worksheet
                    .write_string(row, 1, value)
                    .map_err(|e| GeneratorError::Generation(e.to_string()))?;
                worksheet
                    .write_string(row, 2, description)
                    .map_err(|e| GeneratorError::Generation(e.to_string()))?;
                row += 1;
            }
        }

        if self.freeze_headers() {
            worksheet
                .set_freeze_panes(1, 0)
                .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        }

        if self.add_filters() {
            worksheet
                .autofilter(0, 0, row.saturating_sub(1), 2)
                .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        }

        worksheet
            .set_column_width(0, 20)
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        worksheet
            .set_column_width(1, 20)
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        worksheet
            .set_column_width(2, 40)
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;

        Ok(())
    }

    fn write_enum_headers(
        worksheet: &mut Worksheet,
        header_format: &Format,
    ) -> Result<(), GeneratorError> {
        worksheet
            .write_string_with_format(0, 0, "Enumeration", header_format)
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        worksheet
            .write_string_with_format(0, 1, "Value", header_format)
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        worksheet
            .write_string_with_format(0, 2, "Description", header_format)
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        Ok(())
    }
}
