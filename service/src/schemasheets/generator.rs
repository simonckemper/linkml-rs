//! SchemaSheets format generator - simplified version

use linkml_core::error::{LinkMLError, Result};
use linkml_core::types::{
    ClassDefinition, EnumDefinition, PermissibleValue, PrefixDefinition, SchemaDefinition,
    SlotDefinition, SubsetDefinition, TypeDefinition,
};
use rust_xlsxwriter::{Format, Workbook, Worksheet};
use std::path::Path;

/// Generator for SchemaSheets format Excel files
#[derive(Debug, Clone)]
pub struct SchemaSheetsGenerator {
    pub include_all_metadata: bool,
    pub generate_metadata_sheets: bool,
}

impl SchemaSheetsGenerator {
    pub fn new() -> Self {
        Self {
            include_all_metadata: true,
            generate_metadata_sheets: true,
        }
    }

    pub async fn generate_file(&self, schema: &SchemaDefinition, output_path: &Path) -> Result<()> {
        let mut workbook = Workbook::new();
        
        // Generate main schema sheet
        let sheet = workbook.add_worksheet();
        sheet.set_name("Schema").unwrap();
        
        // Write header
        let header_format = Format::new().set_bold();
        let headers = vec![">", "element_type", "field", "key", "multiplicity", "range", "desc", "is_a", "pattern"];
        for (col, header) in headers.iter().enumerate() {
            sheet.write_with_format(0, col as u16, *header, &header_format).unwrap();
        }
        
        let mut row = 1;
        
        // Write classes
        for (class_name, class_def) in &schema.classes {
            sheet.write(row, 0, class_name).unwrap();
            sheet.write(row, 1, "class").unwrap();
            if let Some(ref desc) = class_def.description {
                sheet.write(row, 6, desc).unwrap();
            }
            if let Some(ref parent) = class_def.is_a {
                sheet.write(row, 7, parent).unwrap();
            }
            row += 1;
            
            // Write attributes
            for (attr_name, attr_def) in &class_def.attributes {
                sheet.write(row, 2, attr_name).unwrap();
                if attr_def.identifier == Some(true) {
                    sheet.write(row, 3, "true").unwrap();
                }
                let mult = match (attr_def.required.unwrap_or(false), attr_def.multivalued.unwrap_or(false)) {
                    (true, false) => "1",
                    (false, false) => "0..1",
                    (true, true) => "1..*",
                    (false, true) => "0..*",
                };
                sheet.write(row, 4, mult).unwrap();
                if let Some(ref range) = attr_def.range {
                    sheet.write(row, 5, range).unwrap();
                }
                if let Some(ref desc) = attr_def.description {
                    sheet.write(row, 6, desc).unwrap();
                }
                if let Some(ref pattern) = attr_def.pattern {
                    sheet.write(row, 8, pattern).unwrap();
                }
                row += 1;
            }
        }
        
        // Write enums
        for (enum_name, enum_def) in &schema.enums {
            sheet.write(row, 0, enum_name).unwrap();
            sheet.write(row, 1, "enum").unwrap();
            if let Some(ref desc) = enum_def.description {
                sheet.write(row, 6, desc).unwrap();
            }
            row += 1;
            
            for pv in &enum_def.permissible_values {
                let (value, desc) = match pv {
                    PermissibleValue::Simple(v) => (v.clone(), None),
                    PermissibleValue::Complex { text, description, .. } => (text.clone(), description.clone()),
                };
                sheet.write(row, 2, &value).unwrap();
                if let Some(ref d) = desc {
                    sheet.write(row, 6, d).unwrap();
                }
                row += 1;
            }
        }
        
        // Write types
        for (type_name, type_def) in &schema.types {
            sheet.write(row, 0, type_name).unwrap();
            sheet.write(row, 1, "type").unwrap();
            if let Some(ref desc) = type_def.description {
                sheet.write(row, 6, desc).unwrap();
            }
            if let Some(ref base_type) = type_def.base_type {
                sheet.write(row, 7, base_type).unwrap();
            }
            if let Some(ref pattern) = type_def.pattern {
                sheet.write(row, 8, pattern).unwrap();
            }
            row += 1;
        }
        
        // Write subsets
        for (subset_name, subset_def) in &schema.subsets {
            sheet.write(row, 0, subset_name).unwrap();
            sheet.write(row, 1, "subset").unwrap();
            if let Some(ref desc) = subset_def.description {
                sheet.write(row, 6, desc).unwrap();
            }
            row += 1;
        }
        
        // Generate metadata sheets
        if self.generate_metadata_sheets {
            // Prefixes sheet
            let prefixes_sheet = workbook.add_worksheet();
            prefixes_sheet.set_name("prefixes").unwrap();
            prefixes_sheet.write_with_format(0, 0, "prefix", &header_format).unwrap();
            prefixes_sheet.write_with_format(0, 1, "uri", &header_format).unwrap();
            let mut row = 1;
            for (prefix, definition) in &schema.prefixes {
                prefixes_sheet.write(row, 0, prefix).unwrap();
                let uri = match definition {
                    PrefixDefinition::Simple(uri) => uri.clone(),
                    PrefixDefinition::Complex { prefix_reference, .. } => prefix_reference.clone().unwrap_or_default(),
                };
                prefixes_sheet.write(row, 1, uri).unwrap();
                row += 1;
            }
            
            // Settings sheet
            let settings_sheet = workbook.add_worksheet();
            settings_sheet.set_name("settings").unwrap();
            settings_sheet.write_with_format(0, 0, "setting", &header_format).unwrap();
            settings_sheet.write_with_format(0, 1, "value", &header_format).unwrap();
            let mut row = 1;
            settings_sheet.write(row, 0, "id").unwrap();
            settings_sheet.write(row, 1, &schema.id).unwrap();
            row += 1;
            settings_sheet.write(row, 0, "name").unwrap();
            settings_sheet.write(row, 1, &schema.name).unwrap();
            row += 1;
            if let Some(ref version) = schema.version {
                settings_sheet.write(row, 0, "version").unwrap();
                settings_sheet.write(row, 1, version).unwrap();
                row += 1;
            }
            if let Some(ref description) = schema.description {
                settings_sheet.write(row, 0, "description").unwrap();
                settings_sheet.write(row, 1, description).unwrap();
            }
        }
        
        workbook.save(output_path).map_err(|e| LinkMLError::Other {
            message: format!("Failed to save Excel file: {e}"),
            source: None,
        })?;
        
        Ok(())
    }
}

impl Default for SchemaSheetsGenerator {
    fn default() -> Self {
        Self::new()
    }
}
