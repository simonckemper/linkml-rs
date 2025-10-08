use crate::generator::traits::{GeneratorError, GeneratorResult};
use crate::utils::safe_cast::usize_to_f64;
use linkml_core::prelude::{ClassDefinition, PermissibleValue, SchemaDefinition, SlotDefinition};
use rust_xlsxwriter::{
    Color, DataValidation, DataValidationRule, ExcelDateTime, Format, FormatBorder, Formula, Note,
    Workbook, Worksheet,
};

use super::super::generator::ExcelGenerator;
use super::super::{cast, pattern};

const SAMPLE_ROW_COUNT: usize = 5;
const DATA_START_ROW: u32 = 3;

impl ExcelGenerator {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn generate_class_sheet(
        &self,
        workbook: &mut Workbook,
        class_name: &str,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
        header_format: &Format,
        required_format: &Format,
        optional_format: &Format,
        type_format: &Format,
    ) -> GeneratorResult<()> {
        let sheet_name = Self::sanitize_sheet_name(class_name);
        let mut worksheet = workbook
            .add_worksheet()
            .set_name(&sheet_name)
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;

        let slots = self.collect_class_slots(class_name, class_def, schema)?;
        if slots.is_empty() {
            return Ok(());
        }

        self.write_headers(&mut worksheet, &slots, header_format)?;
        self.write_type_row(&mut worksheet, &slots, type_format)?;
        self.write_description_row(&mut worksheet, &slots)?;
        self.write_sample_rows(
            &mut worksheet,
            &slots,
            required_format,
            optional_format,
            DATA_START_ROW,
        )?;

        if self.add_validation() {
            Self::add_data_validations(&mut worksheet, &slots, schema, DATA_START_ROW)?;
        }

        if self.freeze_headers() {
            worksheet
                .set_freeze_panes(3, 0)
                .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        }

        if self.add_filters() {
            let last_row = DATA_START_ROW
                + u32::try_from(SAMPLE_ROW_COUNT.saturating_sub(1))
                    .expect("sample row count fits in u32");
            let max_col = cast::usize_to_u16_column(slots.len().saturating_sub(1))?;
            worksheet
                .autofilter(0, 0, last_row, max_col)
                .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        }

        for (i, _) in slots.iter().enumerate() {
            let col_index = cast::usize_to_u16_column(i)?;
            worksheet
                .set_column_width(col_index, 15)
                .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        }

        Ok(())
    }

    fn write_headers(
        &self,
        worksheet: &mut Worksheet,
        slots: &[(String, SlotDefinition)],
        header_format: &Format,
    ) -> Result<(), GeneratorError> {
        for (col, (slot_name, slot_def)) in slots.iter().enumerate() {
            let col = cast::usize_to_u16_column(col)?;
            worksheet
                .write_string_with_format(0, col, slot_name, header_format)
                .map_err(|e| GeneratorError::Generation(e.to_string()))?;

            if let Some(desc) = &slot_def.description {
                let note = Note::new(desc).set_author("LinkML Generator");
                worksheet
                    .insert_note(0, col, &note)
                    .map_err(|e| GeneratorError::Generation(e.to_string()))?;
            }
        }
        Ok(())
    }

    fn write_type_row(
        &self,
        worksheet: &mut Worksheet,
        slots: &[(String, SlotDefinition)],
        type_format: &Format,
    ) -> Result<(), GeneratorError> {
        for (col_idx, (_, slot_def)) in slots.iter().enumerate() {
            let col = cast::usize_to_u16_column(col_idx)?;
            let type_str = format!("<{}>", slot_def.range.as_deref().unwrap_or("string"));
            worksheet
                .write_string_with_format(1, col, &type_str, type_format)
                .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        }
        Ok(())
    }

    fn write_description_row(
        &self,
        worksheet: &mut Worksheet,
        slots: &[(String, SlotDefinition)],
    ) -> Result<(), GeneratorError> {
        let desc_format = Format::new()
            .set_italic()
            .set_font_color(Color::Gray)
            .set_text_wrap()
            .set_border(FormatBorder::Thin);

        for (col_idx, (_, slot_def)) in slots.iter().enumerate() {
            let col = cast::usize_to_u16_column(col_idx)?;
            let description = slot_def.description.as_deref().unwrap_or("");
            worksheet
                .write_string_with_format(2, col, description, &desc_format)
                .map_err(|e| GeneratorError::Generation(e.to_string()))?;
        }
        Ok(())
    }

    fn write_sample_rows(
        &self,
        worksheet: &mut Worksheet,
        slots: &[(String, SlotDefinition)],
        required_format: &Format,
        optional_format: &Format,
        start_row: u32,
    ) -> Result<(), GeneratorError> {
        for row_index in 0..SAMPLE_ROW_COUNT {
            for (col_idx, (slot_name, slot_def)) in slots.iter().enumerate() {
                let col = cast::usize_to_u16_column(col_idx)?;
                let format = if slot_def.required.unwrap_or(false) {
                    required_format
                } else {
                    optional_format
                };

                let sample = Self::generate_sample_value(slot_name, slot_def, row_index);
                worksheet
                    .write_string_with_format(
                        start_row + u32::try_from(row_index).expect("row index fits in u32"),
                        col,
                        &sample,
                        format,
                    )
                    .map_err(|e| GeneratorError::Generation(e.to_string()))?;
            }
        }
        Ok(())
    }

    fn add_data_validations(
        worksheet: &mut Worksheet,
        slots: &[(String, SlotDefinition)],
        schema: &SchemaDefinition,
        start_row: u32,
    ) -> GeneratorResult<()> {
        for (col_index, (_slot_name, slot_def)) in slots.iter().enumerate() {
            let col = cast::usize_to_u16_column(col_index)?;
            let mut has_validation = false;

            if let Some(range) = &slot_def.range
                && let Some(enum_def) = schema.enums.get(range)
            {
                let values: Vec<String> = enum_def
                    .permissible_values
                    .iter()
                    .map(|pv| match pv {
                        PermissibleValue::Simple(s) => s.clone(),
                        PermissibleValue::Complex { text, .. } => text.clone(),
                    })
                    .collect();

                let data_validation = DataValidation::new()
                    .allow_list_strings(
                        &values
                            .iter()
                            .map(std::string::String::as_str)
                            .collect::<Vec<_>>(),
                    )
                    .map_err(|e| GeneratorError::Generation(e.to_string()))?;

                worksheet
                    .add_data_validation(start_row, col, 1_048_575, col, &data_validation)
                    .map_err(|e| GeneratorError::Generation(e.to_string()))?;

                has_validation = true;
            }

            match slot_def.range.as_deref() {
                Some("integer") => {
                    let mut validation = DataValidation::new();

                    if let (Some(min), Some(max)) =
                        (&slot_def.minimum_value, &slot_def.maximum_value)
                    {
                        if let (Some(min_val), Some(max_val)) = (min.as_i64(), max.as_i64()) {
                            let min_i32 = cast::i64_to_i32_validation(min_val)?;
                            let max_i32 = cast::i64_to_i32_validation(max_val)?;
                            validation = validation
                                .allow_whole_number(DataValidationRule::Between(min_i32, max_i32));
                        }
                    } else if let Some(min) = &slot_def.minimum_value {
                        if let Some(min_val) = min.as_i64() {
                            let min_i32 = cast::i64_to_i32_validation(min_val)?;
                            validation = validation.allow_whole_number(
                                DataValidationRule::GreaterThanOrEqualTo(min_i32),
                            );
                        }
                    } else if let Some(max) = &slot_def.maximum_value {
                        if let Some(max_val) = max.as_i64() {
                            let max_i32 = cast::i64_to_i32_validation(max_val)?;
                            validation = validation
                                .allow_whole_number(DataValidationRule::LessThanOrEqualTo(max_i32));
                        }
                    } else {
                        validation = validation
                            .allow_whole_number(DataValidationRule::GreaterThanOrEqualTo(i32::MIN));
                    }

                    validation = validation
                        .set_input_title("Enter an integer")
                        .map_err(|e| GeneratorError::Generation(e.to_string()))?;

                    if let Some(desc) = &slot_def.description {
                        validation = validation
                            .set_input_message(desc)
                            .map_err(|e| GeneratorError::Generation(e.to_string()))?;
                    }

                    worksheet
                        .add_data_validation(start_row, col, 1_048_575, col, &validation)
                        .map_err(|e| GeneratorError::Generation(e.to_string()))?;

                    has_validation = true;
                }
                Some("float" | "double" | "decimal") => {
                    let mut validation = DataValidation::new();

                    if let (Some(min), Some(max)) =
                        (&slot_def.minimum_value, &slot_def.maximum_value)
                    {
                        if let (Some(min_val), Some(max_val)) = (min.as_f64(), max.as_f64()) {
                            validation = validation.allow_decimal_number(
                                DataValidationRule::Between(min_val, max_val),
                            );
                        }
                    } else {
                        validation = validation
                            .allow_decimal_number(DataValidationRule::Between(f64::MIN, f64::MAX));
                    }

                    validation = validation
                        .set_input_title("Enter a decimal number")
                        .map_err(|e| GeneratorError::Generation(e.to_string()))?;

                    worksheet
                        .add_data_validation(start_row, col, 1_048_575, col, &validation)
                        .map_err(|e| GeneratorError::Generation(e.to_string()))?;

                    has_validation = true;
                }
                Some("date") => {
                    let validation = DataValidation::new()
                        .allow_date(DataValidationRule::Between(
                            ExcelDateTime::from_ymd(1900, 1, 1)
                                .expect("LinkML operation should succeed"),
                            ExcelDateTime::from_ymd(2100, 12, 31)
                                .expect("LinkML operation should succeed"),
                        ))
                        .set_input_title("Enter a date")
                        .map_err(|e| GeneratorError::Generation(e.to_string()))?
                        .set_input_message("Format: YYYY-MM-DD")
                        .map_err(|e| GeneratorError::Generation(e.to_string()))?;

                    worksheet
                        .add_data_validation(start_row, col, 1_048_575, col, &validation)
                        .map_err(|e| GeneratorError::Generation(e.to_string()))?;

                    has_validation = true;
                }
                _ => {}
            }

            if let Some(pattern) = &slot_def.pattern {
                let formula = pattern::build_pattern_formula(pattern, col, start_row)?;
                let mut validation = DataValidation::new().allow_custom(Formula::new(formula));

                validation = validation
                    .set_error_title("Pattern mismatch")
                    .map_err(|e| GeneratorError::Generation(e.to_string()))?;
                validation = validation
                    .set_error_message(&format!("Value must match the pattern: {pattern}"))
                    .map_err(|e| GeneratorError::Generation(e.to_string()))?;
                validation = validation
                    .set_input_title("Pattern constraint")
                    .map_err(|e| GeneratorError::Generation(e.to_string()))?;
                validation = validation
                    .set_input_message(&format!("Allowed pattern: {pattern}"))
                    .map_err(|e| GeneratorError::Generation(e.to_string()))?;

                let note_text = format!("Pattern requirement: {pattern}");
                let note = Note::new(&note_text).set_author("LinkML Generator");

                if has_validation {
                    worksheet
                        .insert_note(start_row, col, &note)
                        .map_err(|e| GeneratorError::Generation(e.to_string()))?;
                } else {
                    worksheet
                        .add_data_validation(start_row, col, 1_048_575, col, &validation)
                        .map_err(|e| GeneratorError::Generation(e.to_string()))?;
                    worksheet
                        .insert_note(start_row, col, &note)
                        .map_err(|e| GeneratorError::Generation(e.to_string()))?;
                }
            }
        }

        Ok(())
    }

    fn collect_class_slots(
        &self,
        _class_name: &str,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<Vec<(String, SlotDefinition)>> {
        let mut slots = std::collections::BTreeMap::new();

        if let Some(parent) = &class_def.is_a
            && let Some(parent_class) = schema.classes.get(parent)
        {
            let parent_slots = self.collect_class_slots(parent, parent_class, schema)?;
            for (name, slot) in parent_slots {
                slots.insert(name, slot);
            }
        }

        for slot_name in &class_def.slots {
            if let Some(slot_def) = schema.slots.get(slot_name) {
                slots.insert(slot_name.clone(), slot_def.clone());
            }
        }

        for (attr_name, attr_def) in &class_def.attributes {
            slots.insert(attr_name.clone(), attr_def.clone());
        }

        Ok(slots.into_iter().collect())
    }

    fn generate_sample_value(name: &str, slot: &SlotDefinition, index: usize) -> String {
        match slot.range.as_deref() {
            Some("string") => format!("{name} {}", index + 1),
            Some("integer") => format!("{}", (index + 1) * 10),
            Some("float") => format!("{:.2}", usize_to_f64(index + 1) * std::f64::consts::PI),
            Some("boolean") => if index.is_multiple_of(2) {
                "TRUE"
            } else {
                "FALSE"
            }
            .to_string(),
            Some("date") => format!("2024-01-{:02}", index + 1),
            Some("datetime") => format!("2024-01-{:02}T10:00:00", index + 1),
            _ => format!("Sample {}", index + 1),
        }
    }

    pub(super) fn sanitize_sheet_name(name: &str) -> String {
        let sanitized = name
            .chars()
            .filter(|c| !matches!(c, '\\' | '/' | '?' | '*' | '[' | ']'))
            .collect::<String>();

        if sanitized.len() > 31 {
            sanitized[..31].to_string()
        } else {
            sanitized
        }
    }
}
