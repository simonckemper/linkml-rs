//! Excel generator for LinkML schemas
//!
//! This generator creates Excel workbooks with multiple sheets from LinkML schemas,
//! including data validation rules, formatting, and documentation.
//!
//! ## Features
//!
//! With rust_xlsxwriter v0.89.1, we now have full support for:
//! - Cell comments/notes for inline documentation
//! - Data validation with dropdown lists and constraints
//! - Conditional formatting for visual feedback
//! - Rich formatting options

use super::traits::{Generator, GeneratorError, GeneratorOptions, GeneratorResult, GeneratedOutput};
use linkml_core::prelude::*;
use async_trait::async_trait;
use std::collections::BTreeMap;
use rust_xlsxwriter::{
    DataValidation, DataValidationRule, Format, FormatAlign, FormatBorder, 
    Note, Workbook, Worksheet, Color, XlsxError
};

/// Excel generator
pub struct ExcelGenerator {
    /// Whether to include a summary sheet
    include_summary: bool,
    /// Whether to add data validation
    add_validation: bool,
    /// Whether to freeze header rows
    freeze_headers: bool,
    /// Whether to add filters
    add_filters: bool,
}

impl ExcelGenerator {
    /// Create a new Excel generator
    #[must_use]
    pub fn new() -> Self {
        Self {
            include_summary: true,
            add_validation: true,
            freeze_headers: true,
            add_filters: true,
        }
    }

    /// Configure summary sheet generation
    #[must_use]
    pub fn with_summary(mut self, enabled: bool) -> Self {
        self.include_summary = enabled;
        self
    }

    /// Configure data validation
    #[must_use]
    pub fn with_validation(mut self, enabled: bool) -> Self {
        self.add_validation = enabled;
        self
    }

    /// Configure header freezing
    #[must_use]
    pub fn with_frozen_headers(mut self, enabled: bool) -> Self {
        self.freeze_headers = enabled;
        self
    }

    /// Configure filter addition
    #[must_use]
    pub fn with_filters(mut self, enabled: bool) -> Self {
        self.add_filters = enabled;
        self
    }

    /// Generate the Excel workbook
    fn generate_workbook(&self, schema: &SchemaDefinition) -> GeneratorResult<Vec<u8>> {
        let mut workbook = Workbook::new();
        
        // Create formats
        let header_format = Format::new()
            .set_bold()
            .set_background_color(Color::Gray)
            .set_font_color(Color::White)
            .set_align(FormatAlign::Center)
            .set_border(FormatBorder::Thin);
            
        let required_format = Format::new()
            .set_background_color(Color::RGB(0xFFEBCD))
            .set_border(FormatBorder::Thin);
            
        let optional_format = Format::new()
            .set_border(FormatBorder::Thin);
            
        let type_format = Format::new()
            .set_font_color(Color::Blue)
            .set_italic()
            .set_border(FormatBorder::Thin);
            
        // Generate summary sheet
        if self.include_summary {
            self.generate_summary_sheet(&mut workbook, schema, &header_format)?;
        }
        
        // Generate sheet for each non-abstract class
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
                &type_format
            )?;
        }
        
        // Generate enums sheet
        if !schema.enums.is_empty() {
            self.generate_enums_sheet(&mut workbook, schema, &header_format)?;
        }
        
        // Generate enums reference sheet (used for dropdown lists)
        self.generate_enums_sheet(&mut workbook, schema, &header_format)?
        
        // Convert workbook to bytes
        let buffer = workbook.save_to_buffer()
            .map_err(|e| GeneratorError::Generation {
                context: "save_buffer".to_string(),
                message: format!("Failed to save workbook: {}", e),
            })?;
            
        Ok(buffer)
    }

    /// Generate summary sheet
    fn generate_summary_sheet(&self, 
        workbook: &mut Workbook,
        schema: &SchemaDefinition,
        header_format: &Format
    ) -> GeneratorResult<()> {
        let worksheet = workbook.add_worksheet()
            .set_name("Summary")
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
            
        let mut row = 0;
        
        // Title
        worksheet.write_string(row, 0, if schema.name.is_empty() { "LinkML Schema" } else { &schema.name })
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        worksheet.merge_range(row, 0, row, 3, if schema.name.is_empty() { "LinkML Schema" } else { &schema.name }, header_format)
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        row += 2;
        
        // Description
        if let Some(description) = &schema.description {
            worksheet.write_string(row, 0, "Description:")
                .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
            worksheet.write_string(row, 1, description)
                .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
            row += 2;
        }
        
        // Statistics
        worksheet.write_string_with_format(row, 0, "Statistics", &header_format)
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        worksheet.write_string_with_format(row, 1, "Count", &header_format)
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        row += 1;
        
        worksheet.write_string(row, 0, "Classes")
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        worksheet.write_number(row, 1, schema.classes.len() as f64)
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        row += 1;
        
        worksheet.write_string(row, 0, "Slots")
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        worksheet.write_number(row, 1, schema.slots.len() as f64)
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        row += 1;
        
        worksheet.write_string(row, 0, "Enumerations")
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        worksheet.write_number(row, 1, schema.enums.len() as f64)
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        row += 1;
        
        worksheet.write_string(row, 0, "Types")
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        worksheet.write_number(row, 1, schema.types.len() as f64)
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        
        // Set column widths
        worksheet.set_column_width(0, 20)
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        worksheet.set_column_width(1, 40)
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
            
        Ok(())
    }

    /// Generate sheet for a class
    fn generate_class_sheet(&self,
        workbook: &mut Workbook,
        class_name: &str,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
        header_format: &Format,
        required_format: &Format,
        optional_format: &Format,
        type_format: &Format
    ) -> GeneratorResult<()> {
        // Create worksheet with sanitized name
        let sheet_name = self.sanitize_sheet_name(class_name);
        let worksheet = workbook.add_worksheet()
            .set_name(&sheet_name)
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
            
        // Collect all slots
        let slots = self.collect_class_slots(class_name, class_def, schema)?;
        
        if slots.is_empty() {
            return Ok(());
        }
        
        let mut row = 0;
        let mut col = 0;
        
        // Write headers
        for (slot_name, slot_def) in &slots {
            worksheet.write_string_with_format(row, col, slot_name, &header_format)
                .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
                
            // Add description as a cell note
            if let Some(desc) = &slot_def.description {
                let note = Note::new(desc).set_author("LinkML Generator");
                worksheet.insert_note(row, col, &note)
                    .map_err(|e| GeneratorError::Generation {
                        context: "worksheet_operation".to_string(),
                        message: e.to_string(),
                    })?;
            }
            
            col += 1;
        }
        row += 1;
        
        // Write type row
        col = 0;
        for (_, slot_def) in &slots {
            let type_str = format!("<{}>", slot_def.range.as_deref().unwrap_or("string"));
            worksheet.write_string_with_format(row, col, &type_str, &type_format)
                .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
            col += 1;
        }
        row += 1;
        
        // Write description row (since we can't use cell notes in v0.64)
        col = 0;
        let desc_format = Format::new()
            .set_italic()
            .set_font_color(Color::Gray)
            .set_text_wrap()
            .set_border(FormatBorder::Thin);
            
        for (_, slot_def) in &slots {
            let description = slot_def.description.as_deref().unwrap_or("");
            worksheet.write_string_with_format(row, col, description, &desc_format)
                .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
            col += 1;
        }
        row += 1;
        
        // Add sample data rows
        for i in 0..5 {
            col = 0;
            for (slot_name, slot_def) in &slots {
                let format = if slot_def.required.unwrap_or(false) {
                    required_format
                } else {
                    optional_format
                };
                
                let sample = self.generate_sample_value(slot_name, slot_def, i);
                worksheet.write_string_with_format(row, col, &sample, &format)
                    .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
                col += 1;
            }
            row += 1;
        }
        
        // Add data validation for enum fields and constraints
        if self.add_validation {
            self.add_data_validations(worksheet, &slots, schema, 3)?;
        }
        
        // Freeze headers if enabled (now 3 rows: headers, types, descriptions)
        if self.freeze_headers {
            worksheet.set_freeze_panes(3, 0)
                .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        }
        
        // Add filters if enabled
        if self.add_filters {
            worksheet.autofilter(0, 0, row - 1, slots.len() as u16 - 1)
                .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        }
        
        // Auto-fit columns
        for (i, _) in slots.iter().enumerate() {
            worksheet.set_column_width(i as u16, 15)
                .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        }
        
        Ok(())
    }

    /// Generate enums sheet
    fn generate_enums_sheet(&self,
        workbook: &mut Workbook,
        schema: &SchemaDefinition,
        header_format: &Format
    ) -> GeneratorResult<()> {
        let worksheet = workbook.add_worksheet()
            .set_name("Enumerations")
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
            
        let mut row = 0;
        
        // Headers
        worksheet.write_string_with_format(row, 0, "Enumeration", &header_format)
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        worksheet.write_string_with_format(row, 1, "Value", &header_format)
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        worksheet.write_string_with_format(row, 2, "Description", &header_format)
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        row += 1;
        
        // Write enum values
        for (enum_name, enum_def) in &schema.enums {
            for pv in &enum_def.permissible_values {
                let (value, description) = match pv {
                    linkml_core::types::PermissibleValue::Simple(s) => (s.as_str(), ""),
                    linkml_core::types::PermissibleValue::Complex { text, description, .. } => {
                        (text.as_str(), description.as_deref().unwrap_or(""))
                    }
                };
                
                worksheet.write_string(row, 0, enum_name)
                    .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
                worksheet.write_string(row, 1, value)
                    .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
                worksheet.write_string(row, 2, description)
                    .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
                row += 1;
            }
        }
        
        // Freeze headers
        if self.freeze_headers {
            worksheet.set_freeze_panes(1, 0)
                .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        }
        
        // Add filters
        if self.add_filters {
            worksheet.autofilter(0, 0, row - 1, 2)
                .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        }
        
        // Set column widths
        worksheet.set_column_width(0, 20)
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        worksheet.set_column_width(1, 20)
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        worksheet.set_column_width(2, 40)
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
            
        Ok(())
    }
    
    /// Add data validation to cells
    fn add_data_validations(&self,
        worksheet: &mut Worksheet,
        slots: &[(String, SlotDefinition)],
        schema: &SchemaDefinition,
        start_row: u32
    ) -> GeneratorResult<()> {
        for (col, (slot_name, slot_def)) in slots.iter().enumerate() {
            let col = col as u16;
            
            // Check if this is an enum field
            if let Some(range) = &slot_def.range {
                if let Some(enum_def) = schema.enums.get(range) {
                    // Create dropdown list from enum values
                    let values: Vec<String> = enum_def.permissible_values.iter()
                        .map(|pv| match pv {
                            PermissibleValue::Simple(s) => s.clone(),
                            PermissibleValue::Complex { text, .. } => text.clone(),
                        })
                        .collect();
                    
                    let data_validation = DataValidation::new()
                        .allow_list_strings(&values.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                        .map_err(|e| GeneratorError::Generation {
                            context: "data_validation".to_string(),
                            message: e.to_string(),
                        })?;
                    
                    // Apply to entire column (starting from row 3)
                    worksheet.add_data_validation(start_row, col, 1048575, col, &data_validation)
                        .map_err(|e| GeneratorError::Generation {
                            context: "data_validation".to_string(),
                            message: e.to_string(),
                        })?;
                }
            }
            
            // Add numeric constraints
            match slot_def.range.as_deref() {
                Some("integer") => {
                    let mut validation = DataValidation::new();
                    
                    if let (Some(min), Some(max)) = (&slot_def.minimum_value, &slot_def.maximum_value) {
                        if let (Some(min_val), Some(max_val)) = (min.as_i64(), max.as_i64()) {
                            validation = validation.allow_whole_number(
                                DataValidationRule::Between(min_val as i32, max_val as i32)
                            );
                        }
                    } else if let Some(min) = &slot_def.minimum_value {
                        if let Some(min_val) = min.as_i64() {
                            validation = validation.allow_whole_number(
                                DataValidationRule::GreaterThanOrEqualTo(min_val as i32)
                            );
                        }
                    } else if let Some(max) = &slot_def.maximum_value {
                        if let Some(max_val) = max.as_i64() {
                            validation = validation.allow_whole_number(
                                DataValidationRule::LessThanOrEqualTo(max_val as i32)
                            );
                        }
                    } else {
                        validation = validation.allow_whole_number_formula("TRUE");
                    }
                    
                    validation = validation
                        .set_input_title("Enter an integer")
                        .map_err(|e| GeneratorError::Generation {
                            context: "data_validation".to_string(),
                            message: e.to_string(),
                        })?;
                    
                    if let Some(desc) = &slot_def.description {
                        validation = validation.set_input_message(desc)
                            .map_err(|e| GeneratorError::Generation {
                                context: "data_validation".to_string(),
                                message: e.to_string(),
                            })?;
                    }
                    
                    worksheet.add_data_validation(start_row, col, 1048575, col, &validation)
                        .map_err(|e| GeneratorError::Generation {
                            context: "data_validation".to_string(),
                            message: e.to_string(),
                        })?;
                }
                Some("float") | Some("double") | Some("decimal") => {
                    let mut validation = DataValidation::new();
                    
                    if let (Some(min), Some(max)) = (&slot_def.minimum_value, &slot_def.maximum_value) {
                        if let (Some(min_val), Some(max_val)) = (min.as_f64(), max.as_f64()) {
                            validation = validation.allow_decimal_number(
                                DataValidationRule::Between(min_val, max_val)
                            );
                        }
                    } else {
                        validation = validation.allow_decimal_number_formula("TRUE");
                    }
                    
                    validation = validation
                        .set_input_title("Enter a decimal number")
                        .map_err(|e| GeneratorError::Generation {
                            context: "data_validation".to_string(),
                            message: e.to_string(),
                        })?;
                    
                    worksheet.add_data_validation(start_row, col, 1048575, col, &validation)
                        .map_err(|e| GeneratorError::Generation {
                            context: "data_validation".to_string(),
                            message: e.to_string(),
                        })?;
                }
                Some("date") => {
                    let validation = DataValidation::new()
                        .allow_date_formula("TRUE")
                        .set_input_title("Enter a date")
                        .map_err(|e| GeneratorError::Generation {
                            context: "data_validation".to_string(),
                            message: e.to_string(),
                        })?
                        .set_input_message("Format: YYYY-MM-DD")
                        .map_err(|e| GeneratorError::Generation {
                            context: "data_validation".to_string(),
                            message: e.to_string(),
                        })?;
                    
                    worksheet.add_data_validation(start_row, col, 1048575, col, &validation)
                        .map_err(|e| GeneratorError::Generation {
                            context: "data_validation".to_string(),
                            message: e.to_string(),
                        })?;
                }
                _ => {}
            }
            
            // Add pattern validation if present
            if let Some(pattern) = &slot_def.pattern {
                // Excel doesn't support regex directly, but we can add a custom formula
                // For now, just add a note about the pattern
                // Add pattern information to note
                let note_text = format!("Pattern: {}", pattern);
                let note = Note::new(&note_text).set_author("LinkML Generator");
                worksheet.insert_note(start_row, col, &note)
                    .map_err(|e| GeneratorError::Generation {
                        context: "worksheet_operation".to_string(),
                        message: e.to_string(),
                    })?;
            }
        }
        
        Ok(())
    }
    
    /// DEPRECATED: Generate validation information sheet
    #[allow(dead_code)]
    fn generate_validation_sheet(&self,
        workbook: &mut Workbook,
        schema: &SchemaDefinition,
        header_format: &Format
    ) -> GeneratorResult<()> {
        let worksheet = workbook.add_worksheet()
            .set_name("Validation Info")
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
            
        let mut row = 0;
        
        // Title
        worksheet.write_string(row, 0, "Field Validation Rules")
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        worksheet.merge_range(row, 0, row, 4, "Field Validation Rules", header_format)
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        row += 2;
        
        // Headers
        worksheet.write_string_with_format(row, 0, "Class", &header_format)
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        worksheet.write_string_with_format(row, 1, "Field", &header_format)
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        worksheet.write_string_with_format(row, 2, "Type", &header_format)
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        worksheet.write_string_with_format(row, 3, "Required", &header_format)
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        worksheet.write_string_with_format(row, 4, "Constraints", &header_format)
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        row += 1;
        
        // Write validation rules for each class
        for (class_name, class_def) in &schema.classes {
            if class_def.abstract_.unwrap_or(false) {
                continue;
            }
            
            let slots = self.collect_class_slots(class_name, class_def, schema)?;
            
            for (slot_name, slot_def) in &slots {
                worksheet.write_string(row, 0, class_name)
                    .map_err(|e| GeneratorError::Generation {
                        context: "worksheet_operation".to_string(),
                        message: e.to_string(),
                    })?;
                worksheet.write_string(row, 1, slot_name)
                    .map_err(|e| GeneratorError::Generation {
                        context: "worksheet_operation".to_string(),
                        message: e.to_string(),
                    })?;
                worksheet.write_string(row, 2, slot_def.range.as_deref().unwrap_or("string"))
                    .map_err(|e| GeneratorError::Generation {
                        context: "worksheet_operation".to_string(),
                        message: e.to_string(),
                    })?;
                worksheet.write_string(row, 3, if slot_def.required.unwrap_or(false) { "Yes" } else { "No" })
                    .map_err(|e| GeneratorError::Generation {
                        context: "worksheet_operation".to_string(),
                        message: e.to_string(),
                    })?;
                
                // Build constraints string
                let mut constraints = Vec::new();
                
                if let Some(range) = &slot_def.range {
                    if schema.enums.contains_key(range) {
                        constraints.push(format!("Enum: {}", range));
                    }
                }
                
                if let Some(min) = &slot_def.minimum_value {
                    constraints.push(format!("Min: {}", min));
                }
                
                if let Some(max) = &slot_def.maximum_value {
                    constraints.push(format!("Max: {}", max));
                }
                
                if let Some(pattern) = &slot_def.pattern {
                    constraints.push(format!("Pattern: {}", pattern));
                }
                
                let constraints_str = if constraints.is_empty() {
                    "None".to_string()
                } else {
                    constraints.join("; ")
                };
                
                worksheet.write_string(row, 4, &constraints_str)
                    .map_err(|e| GeneratorError::Generation {
                        context: "worksheet_operation".to_string(),
                        message: e.to_string(),
                    })?;
                    
                row += 1;
            }
        }
        
        // Set column widths
        worksheet.set_column_width(0, 20)
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        worksheet.set_column_width(1, 20)
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        worksheet.set_column_width(2, 15)
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        worksheet.set_column_width(3, 10)
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
        worksheet.set_column_width(4, 40)
            .map_err(|e| GeneratorError::Generation {
                context: "worksheet_operation".to_string(),
                message: e.to_string(),
            })?;
            
        Ok(())
    }

    /// Collect all slots for a class
    fn collect_class_slots(&self,
        _class_name: &str,
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
        
        // Add direct slots
        for slot_name in &class_def.slots {
            if let Some(slot_def) = schema.slots.get(slot_name) {
                slots.insert(slot_name.clone(), slot_def.clone());
            }
        }
        
        // Add attributes
        for (attr_name, attr_def) in &class_def.attributes {
            slots.insert(attr_name.clone(), attr_def.clone());
        }
        
        Ok(slots.into_iter().collect())
    }

    /// Generate sample value
    fn generate_sample_value(&self, name: &str, slot: &SlotDefinition, index: usize) -> String {
        match slot.range.as_deref() {
            Some("string") => format!("{} {}", name, index + 1),
            Some("integer") => format!("{}", (index + 1) * 10),
            Some("float") => format!("{:.2}", (index + 1) as f64 * 3.14),
            Some("boolean") => if index % 2 == 0 { "TRUE" } else { "FALSE" }.to_string(),
            Some("date") => format!("2024-01-{:02}", index + 1),
            Some("datetime") => format!("2024-01-{:02}T10:00:00", index + 1),
            _ => format!("Sample {}", index + 1),
        }
    }

    /// Sanitize sheet name for Excel
    fn sanitize_sheet_name(&self, name: &str) -> String {
        // Excel sheet names have restrictions
        let sanitized = name
            .chars()
            .filter(|c| !matches!(c, '\\' | '/' | '?' | '*' | '[' | ']'))
            .collect::<String>();
            
        // Limit to 31 characters
        if sanitized.len() > 31 {
            sanitized[..31].to_string()
        } else {
            sanitized
        }
    }
}

impl Default for ExcelGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Generator for ExcelGenerator {
    fn name(&self) -> &str {
        "excel"
    }

    fn description(&self) -> &'static str {
        "Generate Excel workbooks from LinkML schemas"
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec![".xlsx"]
    }

    async fn generate(
        &self,
        schema: &SchemaDefinition,
        _options: &GeneratorOptions,
    ) -> GeneratorResult<Vec<GeneratedOutput>> {
        let content = self.generate_workbook(schema)?;
        
        // Convert bytes to string for the interface (base64 encoding)
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(&content);
        
        Ok(vec![GeneratedOutput {
            filename: format!("{}.xlsx", if schema.name.is_empty() { "schema" } else { &schema.name }),
            content: encoded,
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
        schema.description = Some("A test schema for Excel generation".to_string());
        
        // Add a class
        let mut person_class = ClassDefinition::default();
        person_class.slots = vec!["name".to_string(), "age".to_string(), "status".to_string()];
        schema.classes.insert("Person".to_string(), person_class);
        
        // Add slots
        let mut name_slot = SlotDefinition::default();
        name_slot.range = Some("string".to_string());
        name_slot.required = Some(true);
        schema.slots.insert("name".to_string(), name_slot);
        
        let mut age_slot = SlotDefinition::default();
        age_slot.range = Some("integer".to_string());
        age_slot.minimum_value = Some(serde_json::json!(0));
        age_slot.maximum_value = Some(serde_json::json!(150));
        schema.slots.insert("age".to_string(), age_slot);
        
        let mut status_slot = SlotDefinition::default();
        status_slot.range = Some("Status".to_string());
        schema.slots.insert("status".to_string(), status_slot);
        
        // Add enum
        let mut status_enum = EnumDefinition::default();
        status_enum.permissible_values.push(linkml_core::types::PermissibleValue::Simple("ACTIVE".to_string()));
        status_enum.permissible_values.push(linkml_core::types::PermissibleValue::Simple("INACTIVE".to_string()));
        schema.enums.insert("Status".to_string(), status_enum);
        
        schema
    }

    #[tokio::test]
    async fn test_excel_generation() {
        let schema = create_test_schema();
        let generator = ExcelGenerator::new();
        let options = GeneratorOptions::default();
        
        let result = generator.generate(&schema, &options).await.expect("should generate Excel");
        assert_eq!(result.len(), 1);
        
        let output = &result[0];
        assert_eq!(output.filename, "TestSchema.xlsx");
        
        // Content should be base64 encoded
        assert!(!output.content.is_empty());
        
        // Decode and verify it's valid Excel data
        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD.decode(&output.content).expect("should decode base64");
        // Excel files start with specific magic bytes
        assert!(decoded.starts_with(&[0x50, 0x4B])); // ZIP format
    }

    #[test]
    fn test_sheet_name_sanitization() {
        let generator = ExcelGenerator::new();
        
        assert_eq!(generator.sanitize_sheet_name("Simple"), "Simple");
        assert_eq!(generator.sanitize_sheet_name("With/Slash"), "WithSlash");
        assert_eq!(generator.sanitize_sheet_name("With?Question"), "WithQuestion");
        assert_eq!(generator.sanitize_sheet_name("A".repeat(40).as_str()), "A".repeat(31));
    }
}