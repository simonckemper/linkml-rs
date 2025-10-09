use crate::generator::traits::GeneratorError;
use linkml_core::prelude::{SchemaDefinition, SlotDefinition};
use rust_xlsxwriter::{Format, Workbook};

use super::super::generator::ExcelGenerator;

impl ExcelGenerator {
    pub(crate) fn generate_validation_sheet(
        &self,
        workbook: &mut Workbook,
        schema: &SchemaDefinition,
        header_format: &Format,
    ) -> Result<(), GeneratorError> {
        let worksheet = workbook
            .add_worksheet()
            .set_name("Validation Info")
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;

        let mut row = 0;
        worksheet
            .write_string(row, 0, "Field Validation Rules")
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        worksheet
            .merge_range(row, 0, row, 4, "Field Validation Rules", header_format)
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        row += 2;

        self.write_validation_headers(worksheet, row, header_format)?;
        row += 1;

        for (class_name, class_def) in &schema.classes {
            for slot_name in &class_def.slots {
                if let Some(slot_def) = schema.slots.get(slot_name) {
                    self.write_validation_row(
                        worksheet,
                        schema,
                        row,
                        class_name,
                        slot_name,
                        slot_def,
                    )?;
                    row += 1;
                }
            }
        }

        worksheet
            .set_column_width(0, 20)
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        worksheet
            .set_column_width(1, 20)
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        worksheet
            .set_column_width(2, 15)
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        worksheet
            .set_column_width(3, 10)
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        worksheet
            .set_column_width(4, 40)
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;

        Ok(())
    }

    fn write_validation_headers(
        &self,
        worksheet: &mut rust_xlsxwriter::Worksheet,
        row: u32,
        header_format: &Format,
    ) -> Result<(), GeneratorError> {
        worksheet
            .write_string_with_format(row, 0, "Class", header_format)
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        worksheet
            .write_string_with_format(row, 1, "Field", header_format)
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        worksheet
            .write_string_with_format(row, 2, "Type", header_format)
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        worksheet
            .write_string_with_format(row, 3, "Required", header_format)
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        worksheet
            .write_string_with_format(row, 4, "Constraints", header_format)
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        Ok(())
    }

    fn write_validation_row(
        &self,
        worksheet: &mut rust_xlsxwriter::Worksheet,
        schema: &SchemaDefinition,
        row: u32,
        class_name: &str,
        slot_name: &str,
        slot_def: &SlotDefinition,
    ) -> Result<(), GeneratorError> {
        worksheet
            .write_string(row, 0, class_name)
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        worksheet
            .write_string(row, 1, slot_name)
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        worksheet
            .write_string(row, 2, slot_def.range.as_deref().unwrap_or("string"))
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        worksheet
            .write_string(
                row,
                3,
                if slot_def.required.unwrap_or(false) {
                    "Yes"
                } else {
                    "No"
                },
            )
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;

        let mut constraints = Vec::new();

        if let Some(enum_name) = &slot_def.range
            && let Some(enum_def) = schema.enums.get(enum_name)
        {
            let values: Vec<String> = enum_def
                .permissible_values
                .iter()
                .map(|pv| match pv {
                    linkml_core::types::PermissibleValue::Simple(s) => s.clone(),
                    linkml_core::types::PermissibleValue::Complex { text, .. } => text.clone(),
                })
                .collect();
            constraints.push(format!("Enum: {}", values.join(", ")));
        }

        if let Some(min) = &slot_def.minimum_value {
            constraints.push(format!("Min: {min}"));
        }

        if let Some(max) = &slot_def.maximum_value {
            constraints.push(format!("Max: {max}"));
        }

        if let Some(pattern) = &slot_def.pattern {
            constraints.push(format!("Pattern: {pattern}"));
        }

        let constraints_str = if constraints.is_empty() {
            "None".to_string()
        } else {
            constraints.join("; ")
        };

        worksheet
            .write_string(row, 4, &constraints_str)
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;

        Ok(())
    }
}
