use crate::generator::traits::GeneratorError;
use linkml_core::prelude::SchemaDefinition;
use rust_xlsxwriter::{Format, Workbook};

use super::super::generator::ExcelGenerator;

impl ExcelGenerator {
    pub(crate) fn generate_summary_sheet(
        &self,
        workbook: &mut Workbook,
        schema: &SchemaDefinition,
        header_format: &Format,
    ) -> Result<(), GeneratorError> {
        let worksheet = workbook
            .add_worksheet()
            .set_name("Summary")
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;

        let mut row = 0;
        let schema_name = if schema.name.is_empty() {
            "LinkML Schema"
        } else {
            &schema.name
        };

        worksheet
            .write_string(row, 0, schema_name)
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        worksheet
            .merge_range(row, 0, row, 3, schema_name, header_format)
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        row += 2;

        if let Some(description) = &schema.description {
            worksheet
                .write_string(row, 0, "Description:")
                .map_err(|e| GeneratorError::Generation(e.to_string()))?;
            worksheet
                .write_string(row, 1, description)
                .map_err(|e| GeneratorError::Generation(e.to_string()))?;
            row += 2;
        }

        worksheet
            .write_string_with_format(row, 0, "Statistics", header_format)
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        worksheet
            .write_string_with_format(row, 1, "Count", header_format)
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        row += 1;

        worksheet
            .write_string(row, 0, "Classes")
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        worksheet
            .write_number(
                row,
                1,
                f64::from(u32::try_from(schema.classes.len()).unwrap_or(u32::MAX)),
            )
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        row += 1;

        worksheet
            .write_string(row, 0, "Slots")
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        worksheet
            .write_number(
                row,
                1,
                f64::from(u32::try_from(schema.slots.len()).unwrap_or(u32::MAX)),
            )
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        row += 1;

        worksheet
            .write_string(row, 0, "Enumerations")
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        worksheet
            .write_number(
                row,
                1,
                f64::from(u32::try_from(schema.enums.len()).unwrap_or(u32::MAX)),
            )
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        row += 1;

        worksheet
            .write_string(row, 0, "Types")
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        worksheet
            .write_number(
                row,
                1,
                f64::from(u32::try_from(schema.types.len()).unwrap_or(u32::MAX)),
            )
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;

        worksheet
            .set_column_width(0, 20)
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        worksheet
            .set_column_width(1, 40)
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;

        Ok(())
    }
}
