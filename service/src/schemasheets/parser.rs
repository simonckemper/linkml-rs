//! SchemaSheets format parser
//!
//! Parses Excel files in SchemaSheets format into LinkML schemas,
//! preserving all metadata for lossless roundtrip conversion.

use super::types::{ColumnMapping, SchemaSheetRow, SchemaSheetType};
use calamine::{Data, Reader, Xlsx};
use linkml_core::error::{LinkMLError, Result};
use linkml_core::types::{
    ClassDefinition, EnumDefinition, PermissibleValue, PrefixDefinition, SchemaDefinition,
    SlotDefinition, SubsetDefinition, TypeDefinition,
};
use std::io::Cursor;
use std::path::Path;

/// Parser for SchemaSheets format Excel files
pub struct SchemaSheetsParser {
    /// Whether to be strict about format validation
    strict: bool,
}

impl SchemaSheetsParser {
    /// Create a new SchemaSheets parser
    pub fn new() -> Self {
        Self { strict: false }
    }

    /// Create a new strict parser (fails on format violations)
    pub fn new_strict() -> Self {
        Self { strict: true }
    }

    /// Parse a SchemaSheets Excel file into a LinkML schema
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the Excel file
    /// * `schema_id` - Optional schema ID (uses filename if not provided)
    ///
    /// # Returns
    ///
    /// A LinkML `SchemaDefinition` with all metadata preserved
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - File cannot be read
    /// - File is not in SchemaSheets format
    /// - Format is invalid (in strict mode)
    pub async fn parse_file(&self, path: &Path, schema_id: Option<&str>) -> Result<SchemaDefinition> {
        // Read Excel file
        let file_bytes = std::fs::read(path)
            .map_err(|e| LinkMLError::io_error(format!("Failed to read file: {e}")))?;

        let cursor = Cursor::new(file_bytes);
        let mut workbook: Xlsx<_> = Xlsx::new(cursor)
            .map_err(|e| LinkMLError::parse(format!("Failed to parse Excel file: {e}")))?;

        // Determine schema ID
        let schema_id = schema_id.unwrap_or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("schema")
        });

        // Parse all sheets
        let mut schema = SchemaDefinition::default();
        schema.id = schema_id.to_string();
        schema.name = schema_id.to_string();

        // Get sheet names
        let sheet_names = workbook.sheet_names().to_vec();

        for sheet_name in sheet_names {
            // Skip metadata sheets
            if sheet_name.to_lowercase() == "prefixes"
                || sheet_name.to_lowercase() == "types"
                || sheet_name.to_lowercase() == "settings"
            {
                self.parse_metadata_sheet(&mut workbook, &sheet_name, &mut schema)?;
                continue;
            }

            // Parse schema content sheet
            self.parse_schema_sheet(&mut workbook, &sheet_name, &mut schema)?;
        }

        Ok(schema)
    }

    /// Parse a metadata sheet (prefixes, types, settings)
    fn parse_metadata_sheet(
        &self,
        workbook: &mut Xlsx<Cursor<Vec<u8>>>,
        sheet_name: &str,
        schema: &mut SchemaDefinition,
    ) -> Result<()> {
        let range = workbook
            .worksheet_range(sheet_name)
            .map_err(|e| LinkMLError::parse(format!("Failed to read sheet '{sheet_name}': {e}")))?;

        match sheet_name.to_lowercase().as_str() {
            "prefixes" => self.parse_prefixes_sheet(&range, schema),
            "types" => self.parse_types_sheet(&range, schema),
            "settings" => self.parse_settings_sheet(&range, schema),
            _ => Ok(()),
        }
    }

    /// Parse prefixes sheet
    fn parse_prefixes_sheet(
        &self,
        range: &calamine::Range<Data>,
        schema: &mut SchemaDefinition,
    ) -> Result<()> {
        // Prefixes sheet format:
        // | prefix | uri |
        // | schema | http://schema.org/ |
        // | skos   | http://www.w3.org/2004/02/skos/core# |

        let rows: Vec<Vec<Data>> = range.rows().map(|row| row.to_vec()).collect();
        if rows.is_empty() {
            return Ok(());
        }

        // Skip header row
        for row in rows.iter().skip(1) {
            if row.len() < 2 {
                continue;
            }

            let prefix = self.data_to_string(&row[0]);
            let uri = self.data_to_string(&row[1]);

            if !prefix.is_empty() && !uri.is_empty() {
                schema.prefixes.insert(prefix, PrefixDefinition::Simple(uri));
            }
        }

        Ok(())
    }

    /// Parse types sheet
    fn parse_types_sheet(
        &self,
        range: &calamine::Range<Data>,
        schema: &mut SchemaDefinition,
    ) -> Result<()> {
        // Types sheet format similar to main schema sheet
        // Parse custom type definitions from the types sheet

        // Get header row to find column indices
        let headers = self.get_headers(range)?;

        for (_row_idx, row) in range.rows().enumerate().skip(1) {
            let element_name = self.get_cell_value(row, &headers, ">")?;
            let element_type = self.get_cell_value(row, &headers, "element_type")?;

            if element_type == "type" && !element_name.is_empty() {
                let mut type_def = TypeDefinition {
                    name: element_name.clone(),
                    ..Default::default()
                };

                // Parse description
                if let Ok(desc) = self.get_cell_value(row, &headers, "desc") {
                    if !desc.is_empty() {
                        type_def.description = Some(desc);
                    }
                }

                // Parse base type (is_a column)
                if let Ok(base_type) = self.get_cell_value(row, &headers, "is_a") {
                    if !base_type.is_empty() {
                        type_def.base_type = Some(base_type);
                    }
                }

                // Parse pattern
                if let Ok(pattern) = self.get_cell_value(row, &headers, "pattern") {
                    if !pattern.is_empty() {
                        type_def.pattern = Some(pattern);
                    }
                }

                schema.types.insert(element_name, type_def);
            }
        }

        Ok(())
    }

    /// Parse settings sheet
    fn parse_settings_sheet(
        &self,
        range: &calamine::Range<Data>,
        schema: &mut SchemaDefinition,
    ) -> Result<()> {
        // Settings sheet format:
        // | setting | value |
        // | id      | https://example.org/schema |
        // | version | 1.0.0 |
        // | description | My schema |

        let rows: Vec<Vec<Data>> = range.rows().map(|row| row.to_vec()).collect();
        if rows.is_empty() {
            return Ok(());
        }

        // Skip header row
        for row in rows.iter().skip(1) {
            if row.len() < 2 {
                continue;
            }

            let setting = self.data_to_string(&row[0]).to_lowercase();
            let value = self.data_to_string(&row[1]);

            if value.is_empty() {
                continue;
            }

            match setting.as_str() {
                "id" => schema.id = value,
                "name" => schema.name = value,
                "version" => schema.version = Some(value),
                "description" => schema.description = Some(value),
                "license" => schema.license = Some(value),
                _ => {
                    // Unknown setting - log for debugging but don't fail
                    // SchemaDefinition doesn't have a generic metadata field,
                    // so we skip unknown settings gracefully
                    eprintln!("Warning: Unknown setting '{}' with value '{}' in settings sheet", setting, value);
                }
            }
        }

        Ok(())
    }

    /// Parse a schema content sheet (classes, slots, enums)
    fn parse_schema_sheet(
        &self,
        workbook: &mut Xlsx<Cursor<Vec<u8>>>,
        sheet_name: &str,
        schema: &mut SchemaDefinition,
    ) -> Result<()> {
        let range = workbook
            .worksheet_range(sheet_name)
            .map_err(|e| LinkMLError::parse(format!("Failed to read sheet '{sheet_name}': {e}")))?;

        let rows: Vec<Vec<Data>> = range.rows().map(|row| row.to_vec()).collect();
        if rows.is_empty() {
            return Ok(());
        }

        // First row is headers
        let headers: Vec<String> = rows[0].iter().map(|d| self.data_to_string(d)).collect();

        // Create column mapping
        let col_mapping = ColumnMapping::from_headers(&headers);

        // Validate format
        if !col_mapping.is_schemasheets_format() {
            if self.strict {
                return Err(LinkMLError::parse(format!(
                    "Sheet '{}' is not in SchemaSheets format (missing required columns)",
                    sheet_name
                )));
            } else {
                // Skip non-SchemaSheets sheets in non-strict mode
                return Ok(());
            }
        }

        // Parse rows
        let mut current_class: Option<String> = None;
        let mut class_def: Option<ClassDefinition> = None;
        let mut current_enum: Option<String> = None;
        let mut enum_def: Option<EnumDefinition> = None;
        let mut current_type: Option<String> = None;
        let mut type_def: Option<TypeDefinition> = None;

        for row in rows.iter().skip(1) {
            let mut parsed_row = self.parse_row(row, &col_mapping)?;

            // Adjust row type based on context
            if parsed_row.row_type == SchemaSheetType::AttributeDefinition {
                // If we're in an enum context, this is an enum value
                if current_enum.is_some() {
                    parsed_row.row_type = SchemaSheetType::EnumValue;
                }
            }

            match parsed_row.row_type {
                SchemaSheetType::ClassDefinition => {
                    // Save previous class if exists
                    if let (Some(class_name), Some(class)) = (current_class.take(), class_def.take()) {
                        schema.classes.insert(class_name, class);
                    }
                    // Save previous enum if exists
                    if let (Some(enum_name), Some(enum_)) = (current_enum.take(), enum_def.take()) {
                        schema.enums.insert(enum_name, enum_);
                    }
                    // Save previous type if exists
                    if let (Some(type_name), Some(type_)) = (current_type.take(), type_def.take()) {
                        schema.types.insert(type_name, type_);
                    }

                    // Start new class
                    if let Some(class_name) = parsed_row.class_name.clone() {
                        current_class = Some(class_name.clone());
                        class_def = Some(self.create_class_definition(&parsed_row));
                    }
                }
                SchemaSheetType::AttributeDefinition => {
                    // Add attribute to current class
                    if let Some(ref mut class) = class_def {
                        if let Some(field_name) = parsed_row.field_name.clone() {
                            let slot = self.create_slot_definition(&parsed_row);
                            class.attributes.insert(field_name, slot);
                        }
                    }
                }
                SchemaSheetType::EnumDefinition => {
                    // Save previous class if exists
                    if let (Some(class_name), Some(class)) = (current_class.take(), class_def.take()) {
                        schema.classes.insert(class_name, class);
                    }
                    // Save previous enum if exists
                    if let (Some(enum_name), Some(enum_)) = (current_enum.take(), enum_def.take()) {
                        schema.enums.insert(enum_name, enum_);
                    }
                    // Save previous type if exists
                    if let (Some(type_name), Some(type_)) = (current_type.take(), type_def.take()) {
                        schema.types.insert(type_name, type_);
                    }

                    // Start new enum
                    if let Some(enum_name) = parsed_row.class_name.clone() {
                        current_enum = Some(enum_name.clone());
                        enum_def = Some(self.create_enum_definition(&parsed_row));
                    }
                }
                SchemaSheetType::EnumValue => {
                    // Add value to current enum
                    if let Some(ref mut enum_) = enum_def {
                        if let Some(value) = parsed_row.field_name.clone() {
                            let pv = self.create_permissible_value(&parsed_row, &value);
                            enum_.permissible_values.push(pv);
                        }
                    }
                }
                SchemaSheetType::TypeDefinition => {
                    // Save previous class if exists
                    if let (Some(class_name), Some(class)) = (current_class.take(), class_def.take()) {
                        schema.classes.insert(class_name, class);
                    }
                    // Save previous enum if exists
                    if let (Some(enum_name), Some(enum_)) = (current_enum.take(), enum_def.take()) {
                        schema.enums.insert(enum_name, enum_);
                    }
                    // Save previous type if exists
                    if let (Some(type_name), Some(type_)) = (current_type.take(), type_def.take()) {
                        schema.types.insert(type_name, type_);
                    }

                    // Start new type
                    if let Some(type_name) = parsed_row.class_name.clone() {
                        current_type = Some(type_name.clone());
                        type_def = Some(self.create_type_definition(&parsed_row));
                    }
                }
                SchemaSheetType::SubsetDefinition => {
                    // Handle subset definition
                    if let Some(subset_name) = parsed_row.class_name.clone() {
                        let subset = self.create_subset_definition(&parsed_row);
                        schema.subsets.insert(subset_name, subset);
                    }
                }
                SchemaSheetType::Empty => {
                    // Skip empty rows
                }
            }
        }

        // Save last class/enum/type
        if let (Some(class_name), Some(class)) = (current_class, class_def) {
            schema.classes.insert(class_name, class);
        }
        if let (Some(enum_name), Some(enum_)) = (current_enum, enum_def) {
            schema.enums.insert(enum_name, enum_);
        }
        if let (Some(type_name), Some(type_)) = (current_type, type_def) {
            schema.types.insert(type_name, type_);
        }

        Ok(())
    }

    /// Parse a single row into SchemaSheetRow
    fn parse_row(&self, row: &[Data], col_mapping: &ColumnMapping) -> Result<SchemaSheetRow> {
        let mut parsed = SchemaSheetRow::new();

        // Extract element type first (if present)
        if let Some(idx) = col_mapping.element_type_col {
            if let Some(cell) = row.get(idx) {
                let value = self.data_to_string(cell);
                if !value.is_empty() {
                    parsed.element_type = Some(value.to_lowercase());
                }
            }
        }

        // Extract class name
        if let Some(idx) = col_mapping.class_col {
            if let Some(cell) = row.get(idx) {
                let value = self.data_to_string(cell);
                if !value.is_empty() {
                    parsed.class_name = Some(value);

                    // Determine row type based on element_type or default to class
                    parsed.row_type = if let Some(ref elem_type) = parsed.element_type {
                        match elem_type.as_str() {
                            "enum" => SchemaSheetType::EnumDefinition,
                            "type" | "typedef" => SchemaSheetType::TypeDefinition,
                            "subset" => SchemaSheetType::SubsetDefinition,
                            _ => SchemaSheetType::ClassDefinition,
                        }
                    } else {
                        SchemaSheetType::ClassDefinition
                    };
                }
            }
        }

        // Extract field name
        if let Some(idx) = col_mapping.field_col {
            if let Some(cell) = row.get(idx) {
                let value = self.data_to_string(cell);
                if !value.is_empty() {
                    parsed.field_name = Some(value);
                    if parsed.row_type == SchemaSheetType::Empty {
                        // Determine if this is an enum value or attribute
                        // If we're in an enum context (no element_type column or parent is enum),
                        // treat as enum value, otherwise as attribute
                        parsed.row_type = SchemaSheetType::AttributeDefinition;
                    } else if parsed.row_type == SchemaSheetType::EnumDefinition {
                        // This is an enum value row (has field but parent is enum)
                        parsed.row_type = SchemaSheetType::EnumValue;
                    }
                }
            }
        }

        // Extract other fields
        self.extract_fields(&mut parsed, row, col_mapping);

        Ok(parsed)
    }

    /// Extract all fields from row into SchemaSheetRow
    fn extract_fields(&self, parsed: &mut SchemaSheetRow, row: &[Data], col_mapping: &ColumnMapping) {
        // Key/identifier
        if let Some(idx) = col_mapping.key_col {
            if let Some(cell) = row.get(idx) {
                parsed.is_key = self.parse_boolean(cell);
            }
        }

        // Multiplicity
        if let Some(idx) = col_mapping.multiplicity_col {
            if let Some(cell) = row.get(idx) {
                let value = self.data_to_string(cell);
                if !value.is_empty() {
                    parsed.multiplicity = Some(value);
                }
            }
        }

        // Range
        if let Some(idx) = col_mapping.range_col {
            if let Some(cell) = row.get(idx) {
                let value = self.data_to_string(cell);
                if !value.is_empty() {
                    parsed.range = Some(value);
                }
            }
        }

        // Description
        if let Some(idx) = col_mapping.desc_col {
            if let Some(cell) = row.get(idx) {
                let value = self.data_to_string(cell);
                if !value.is_empty() {
                    parsed.description = Some(value);
                }
            }
        }

        // is_a (inheritance)
        if let Some(idx) = col_mapping.is_a_col {
            if let Some(cell) = row.get(idx) {
                let value = self.data_to_string(cell);
                if !value.is_empty() {
                    parsed.is_a = Some(value);
                }
            }
        }

        // Mixin
        if let Some(idx) = col_mapping.mixin_col {
            if let Some(cell) = row.get(idx) {
                let value = self.data_to_string(cell);
                if !value.is_empty() {
                    parsed.mixins = value.split(',').map(|s| s.trim().to_string()).collect();
                }
            }
        }

        // Required
        if let Some(idx) = col_mapping.required_col {
            if let Some(cell) = row.get(idx) {
                parsed.required = Some(self.parse_boolean(cell));
            }
        }

        // Pattern
        if let Some(idx) = col_mapping.pattern_col {
            if let Some(cell) = row.get(idx) {
                let value = self.data_to_string(cell);
                if !value.is_empty() {
                    parsed.pattern = Some(value);
                }
            }
        }

        // Minimum value
        if let Some(idx) = col_mapping.min_value_col {
            if let Some(cell) = row.get(idx) {
                let value = self.data_to_string(cell);
                if !value.is_empty() {
                    parsed.minimum_value = Some(value);
                }
            }
        }

        // Maximum value
        if let Some(idx) = col_mapping.max_value_col {
            if let Some(cell) = row.get(idx) {
                let value = self.data_to_string(cell);
                if !value.is_empty() {
                    parsed.maximum_value = Some(value);
                }
            }
        }

        // Mappings
        for (mapping_name, idx) in &col_mapping.mapping_cols {
            if let Some(cell) = row.get(*idx) {
                let value = self.data_to_string(cell);
                if !value.is_empty() {
                    parsed.mappings.insert(mapping_name.clone(), value);
                }
            }
        }
    }

    /// Create ClassDefinition from SchemaSheetRow
    fn create_class_definition(&self, row: &SchemaSheetRow) -> ClassDefinition {
        let mut class = ClassDefinition::default();

        if let Some(ref name) = row.class_name {
            class.name = name.clone();
        }

        if let Some(ref desc) = row.description {
            class.description = Some(desc.clone());
        }

        if let Some(ref parent) = row.is_a {
            class.is_a = Some(parent.clone());
        }

        if !row.mixins.is_empty() {
            class.mixins = row.mixins.clone();
        }

        // Handle mappings
        for (mapping_type, mapping_value) in &row.mappings {
            let mapping_type_lower = mapping_type.to_lowercase();

            if mapping_type_lower.contains("exact") {
                class.exact_mappings.push(mapping_value.clone());
            } else if mapping_type_lower.contains("close") {
                class.close_mappings.push(mapping_value.clone());
            } else if mapping_type_lower.contains("related") {
                class.related_mappings.push(mapping_value.clone());
            } else if mapping_type_lower.contains("narrow") {
                class.narrow_mappings.push(mapping_value.clone());
            } else if mapping_type_lower.contains("broad") {
                class.broad_mappings.push(mapping_value.clone());
            } else {
                // Default to exact mapping if type is unclear
                class.exact_mappings.push(mapping_value.clone());
            }
        }

        class
    }

    /// Create EnumDefinition from SchemaSheetRow
    fn create_enum_definition(&self, row: &SchemaSheetRow) -> EnumDefinition {
        let mut enum_def = EnumDefinition::default();

        if let Some(ref name) = row.class_name {
            enum_def.name = name.clone();
        }

        if let Some(ref desc) = row.description {
            enum_def.description = Some(desc.clone());
        }

        // Permissible values will be added as we parse enum value rows
        enum_def.permissible_values = Vec::new();

        enum_def
    }

    /// Create PermissibleValue from SchemaSheetRow
    fn create_permissible_value(&self, row: &SchemaSheetRow, value: &str) -> PermissibleValue {
        // Check if we have description or meaning (complex value)
        if row.description.is_some() || !row.mappings.is_empty() {
            PermissibleValue::Complex {
                text: value.to_string(),
                description: row.description.clone(),
                meaning: row.mappings.get("meaning").cloned()
                    .or_else(|| row.mappings.values().next().cloned()),
            }
        } else {
            PermissibleValue::Simple(value.to_string())
        }
    }

    /// Create TypeDefinition from SchemaSheetRow
    fn create_type_definition(&self, row: &SchemaSheetRow) -> TypeDefinition {
        let mut type_def = TypeDefinition::default();

        if let Some(ref name) = row.class_name {
            type_def.name = name.clone();
        }

        if let Some(ref desc) = row.description {
            type_def.description = Some(desc.clone());
        }

        // Set base_type (parent type) from is_a
        if let Some(ref parent) = row.is_a {
            type_def.base_type = Some(parent.clone());
        }

        // Set pattern
        if let Some(ref pattern) = row.pattern {
            type_def.pattern = Some(pattern.clone());
        }

        // Set minimum/maximum values
        if let Some(ref min) = row.minimum_value {
            // Try to parse as JSON value
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(min) {
                type_def.minimum_value = Some(val);
            }
        }

        if let Some(ref max) = row.maximum_value {
            // Try to parse as JSON value
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(max) {
                type_def.maximum_value = Some(val);
            }
        }

        type_def
    }

    /// Create SubsetDefinition from SchemaSheetRow
    fn create_subset_definition(&self, row: &SchemaSheetRow) -> SubsetDefinition {
        let mut subset = SubsetDefinition::default();

        if let Some(ref name) = row.class_name {
            subset.name = name.clone();
        }

        if let Some(ref desc) = row.description {
            subset.description = Some(desc.clone());
        }

        subset
    }

    /// Create SlotDefinition from SchemaSheetRow
    fn create_slot_definition(&self, row: &SchemaSheetRow) -> SlotDefinition {
        let mut slot = SlotDefinition::default();

        if let Some(ref name) = row.field_name {
            slot.name = name.clone();
        }

        if let Some(ref desc) = row.description {
            slot.description = Some(desc.clone());
        }

        if let Some(ref range) = row.range {
            slot.range = Some(range.clone());
        }

        // Set identifier
        if row.is_key {
            slot.identifier = Some(true);
        }

        // Set required
        if row.is_required_field() {
            slot.required = Some(true);
        }

        // Set multivalued
        if row.is_multivalued() {
            slot.multivalued = Some(true);
        }

        // Set pattern
        if let Some(ref pattern) = row.pattern {
            slot.pattern = Some(pattern.clone());
        }

        // Set minimum/maximum values
        if let Some(ref min) = row.minimum_value {
            if let Ok(val) = min.parse::<i64>() {
                slot.minimum_value = Some(serde_json::json!(val));
            } else if let Ok(val) = min.parse::<f64>() {
                slot.minimum_value = Some(serde_json::json!(val));
            }
        }

        if let Some(ref max) = row.maximum_value {
            if let Ok(val) = max.parse::<i64>() {
                slot.maximum_value = Some(serde_json::json!(val));
            } else if let Ok(val) = max.parse::<f64>() {
                slot.maximum_value = Some(serde_json::json!(val));
            }
        }

        // Handle mappings
        for (mapping_type, mapping_value) in &row.mappings {
            let mapping_type_lower = mapping_type.to_lowercase();

            if mapping_type_lower.contains("exact") {
                slot.exact_mappings.push(mapping_value.clone());
            } else if mapping_type_lower.contains("close") {
                slot.close_mappings.push(mapping_value.clone());
            } else if mapping_type_lower.contains("related") {
                slot.related_mappings.push(mapping_value.clone());
            } else if mapping_type_lower.contains("narrow") {
                slot.narrow_mappings.push(mapping_value.clone());
            } else if mapping_type_lower.contains("broad") {
                slot.broad_mappings.push(mapping_value.clone());
            } else {
                // Default to exact mapping if type is unclear
                slot.exact_mappings.push(mapping_value.clone());
            }
        }

        slot
    }

    /// Get headers from the first row of a range
    fn get_headers(&self, range: &calamine::Range<Data>) -> Result<std::collections::HashMap<String, usize>> {
        let mut headers = std::collections::HashMap::new();

        if let Some(first_row) = range.rows().next() {
            for (col_idx, cell) in first_row.iter().enumerate() {
                let header = self.data_to_string(cell);
                if !header.is_empty() {
                    headers.insert(header, col_idx);
                }
            }
        }

        Ok(headers)
    }

    /// Get cell value from a row by column name
    fn get_cell_value(&self, row: &[Data], headers: &std::collections::HashMap<String, usize>, column_name: &str) -> Result<String> {
        if let Some(&col_idx) = headers.get(column_name) {
            if col_idx < row.len() {
                return Ok(self.data_to_string(&row[col_idx]));
            }
        }
        Ok(String::new())
    }

    /// Convert Data to String
    fn data_to_string(&self, data: &Data) -> String {
        match data {
            Data::Int(i) => i.to_string(),
            Data::Float(f) => f.to_string(),
            Data::String(s) => s.trim().to_string(),
            Data::Bool(b) => b.to_string(),
            Data::DateTime(dt) => format!("{dt:?}"),
            Data::DateTimeIso(dt) => dt.to_string(),
            Data::DurationIso(d) => d.to_string(),
            Data::Error(e) => format!("{e:?}"),
            Data::Empty => String::new(),
        }
    }

    /// Parse boolean from Data
    fn parse_boolean(&self, data: &Data) -> bool {
        match data {
            Data::Bool(b) => *b,
            Data::String(s) => {
                let s_lower = s.trim().to_lowercase();
                s_lower == "true" || s_lower == "yes" || s_lower == "1" || s_lower == "y"
            }
            Data::Int(i) => *i != 0,
            _ => false,
        }
    }
}

impl Default for SchemaSheetsParser {
    fn default() -> Self {
        Self::new()
    }
}

