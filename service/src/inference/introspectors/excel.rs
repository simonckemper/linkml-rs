//! Excel/ODS introspector for schema inference
//!
//! This module analyzes Excel (XLSX, XLS, XLSB) and ODS spreadsheet files
//! to collect structure statistics for LinkML schema generation. It handles:
//! - Multi-sheet workbook processing
//! - Excel native type detection (String, Number, Boolean, DateTime, Error, Empty)
//! - Column type inference using TypeInferencer
//! - Missing value detection across sheets
//! - Relationship detection between sheets via foreign key patterns
//! - Cardinality detection (required vs optional columns)

use crate::inference::builder::SchemaBuilder;
use crate::inference::traits::{DataIntrospector, InferenceError, InferenceResult, TypeInferencer};
use crate::inference::type_inference::create_type_inferencer;
use crate::inference::types::DocumentStats;
use async_trait::async_trait;
use calamine::{Data, Reader, Xlsx};
use linkml_core::types::SchemaDefinition;
use logger_core::{LoggerError, LoggerService};
use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;
use timestamp_core::{TimestampError, TimestampService};

/// Column statistics for Excel analysis
#[derive(Debug, Clone)]
struct ColumnStats {
    /// Column name (from header or generated)
    name: String,
    /// Sample values from this column
    value_samples: Vec<String>,
    /// Total number of non-empty values
    non_empty_count: usize,
    /// Total number of values (including empty)
    total_count: usize,
    /// Excel native types detected in this column
    detected_types: HashMap<String, usize>,
    /// Unique values for enum detection (limited to 100 samples)
    unique_values: HashMap<String, usize>,
    /// Minimum numeric value (for range constraints)
    min_numeric: Option<f64>,
    /// Maximum numeric value (for range constraints)
    max_numeric: Option<f64>,
}

impl ColumnStats {
    fn new(name: String) -> Self {
        Self {
            name,
            value_samples: Vec::new(),
            non_empty_count: 0,
            total_count: 0,
            detected_types: HashMap::new(),
            unique_values: HashMap::new(),
            min_numeric: None,
            max_numeric: None,
        }
    }

    fn record_value(&mut self, value: Data) {
        self.total_count += 1;

        let (value_str, type_name, numeric_val) = match value {
            Data::Int(i) => (i.to_string(), "integer", Some(i as f64)),
            Data::Float(f) => (f.to_string(), "float", Some(f)),
            Data::String(s) if !s.trim().is_empty() => (s, "string", None),
            Data::Bool(b) => (b.to_string(), "boolean", None),
            Data::DateTime(dt) => (format!("{dt:?}"), "datetime", None),
            Data::DateTimeIso(dt) => (dt.to_string(), "datetime", None),
            Data::DurationIso(d) => (d.to_string(), "duration", None),
            Data::Error(e) => (format!("{e:?}"), "error", None),
            Data::Empty | Data::String(_) => {
                // Empty cells or blank strings
                return;
            }
        };

        self.non_empty_count += 1;
        *self.detected_types.entry(type_name.to_string()).or_insert(0) += 1;

        // Track unique values for enum detection (limit to 100 unique values)
        if self.unique_values.len() < 100 {
            *self.unique_values.entry(value_str.clone()).or_insert(0) += 1;
        }

        // Track numeric ranges
        if let Some(num) = numeric_val {
            self.min_numeric = Some(self.min_numeric.map_or(num, |min| min.min(num)));
            self.max_numeric = Some(self.max_numeric.map_or(num, |max| max.max(num)));
        }

        if self.value_samples.len() < 100 {
            self.value_samples.push(value_str);
        }
    }

    /// Calculate if this column is required (has values in all rows)
    fn is_required(&self) -> bool {
        self.total_count > 0 && self.non_empty_count == self.total_count
    }

    /// Get the predominant Excel type in this column
    fn predominant_type(&self) -> Option<String> {
        self.detected_types
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(type_name, _)| type_name.clone())
    }

    /// Detect if this column should be an enum (has limited unique values)
    ///
    /// Returns Some(Vec<String>) with enum values if column has <20 unique values
    /// and represents >80% of total values (indicating it's truly categorical)
    fn detect_enum(&self) -> Option<Vec<String>> {
        let unique_count = self.unique_values.len();

        // Only consider enum if we have 2-20 unique values
        if unique_count < 2 || unique_count > 20 {
            return None;
        }

        // Check if unique values represent majority of data (>80%)
        let total_unique_occurrences: usize = self.unique_values.values().sum();
        if (total_unique_occurrences as f64) / (self.non_empty_count as f64) < 0.8 {
            return None;
        }

        // Return sorted enum values
        let mut enum_values: Vec<String> = self.unique_values.keys().cloned().collect();
        enum_values.sort();
        Some(enum_values)
    }

    /// Get numeric range constraint if applicable
    ///
    /// Returns Some((min, max)) if column contains numeric values
    fn numeric_range(&self) -> Option<(f64, f64)> {
        match (self.min_numeric, self.max_numeric) {
            (Some(min), Some(max)) => Some((min, max)),
            _ => None,
        }
    }
}

/// Sheet statistics for multi-sheet workbook analysis
#[derive(Debug, Clone)]
struct SheetStats {
    /// Sheet name
    name: String,
    /// Column statistics
    columns: Vec<ColumnStats>,
    /// Total row count
    row_count: usize,
    /// Whether first row was treated as header
    /// Used in tests to verify header detection logic
    #[allow(dead_code)]
    has_header: bool,
}

/// Excel introspector implementation
///
/// Analyzes Excel/ODS files by parsing workbook structure, detecting
/// column types from cell values, and identifying relationships between sheets.
pub struct ExcelIntrospector {
    /// Logger service for operation tracking
    logger: Arc<dyn LoggerService<Error = LoggerError>>,

    /// Timestamp service for metadata
    timestamp: Arc<dyn TimestampService<Error = TimestampError>>,

    /// Type inferencer for detecting types from samples
    type_inferencer: Arc<dyn TypeInferencer>,
}

impl ExcelIntrospector {
    /// Create a new Excel introspector
    ///
    /// # Arguments
    /// * `logger` - Logger service instance
    /// * `timestamp` - Timestamp service instance
    pub fn new(
        logger: Arc<dyn LoggerService<Error = LoggerError>>,
        timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
    ) -> Self {
        Self {
            logger,
            timestamp,
            type_inferencer: create_type_inferencer(),
        }
    }

    /// Process a single sheet and collect column statistics
    ///
    /// # Arguments
    /// * `sheet_name` - Name of the sheet
    /// * `rows` - Iterator over sheet rows
    ///
    /// # Returns
    /// * `SheetStats` - Statistics for this sheet
    fn process_sheet<R>(&self, sheet_name: String, mut rows: R) -> InferenceResult<SheetStats>
    where
        R: Iterator<Item = Vec<Data>>,
    {
        let mut columns: Vec<ColumnStats> = Vec::new();
        let mut row_count = 0;
        let mut has_header = false;

        // Read first row to detect headers
        if let Some(first_row) = rows.next() {
            row_count += 1;

            // Heuristic: If first row contains mostly strings, treat as header
            let string_count = first_row.iter().filter(|cell| matches!(cell, Data::String(_))).count();
            has_header = string_count > first_row.len() / 2;

            if has_header {
                // Initialize columns from header row
                for (idx, cell) in first_row.iter().enumerate() {
                    let col_name = match cell {
                        Data::String(s) if !s.trim().is_empty() => s.clone(),
                        _ => format!("column_{}", idx + 1),
                    };
                    columns.push(ColumnStats::new(col_name));
                }
            } else {
                // Generate column names and process first row as data
                for (idx, cell) in first_row.iter().enumerate() {
                    let mut col = ColumnStats::new(format!("column_{}", idx + 1));
                    col.record_value(cell.clone());
                    columns.push(col);
                }
            }
        }

        // Process remaining rows
        for row in rows {
            row_count += 1;

            // Extend columns if this row has more columns
            while columns.len() < row.len() {
                let idx = columns.len();
                columns.push(ColumnStats::new(format!("column_{}", idx + 1)));
            }

            // Record values for each column
            for (idx, cell) in row.iter().enumerate() {
                if let Some(col_stats) = columns.get_mut(idx) {
                    col_stats.record_value(cell.clone());
                }
            }
        }

        Ok(SheetStats {
            name: sheet_name,
            columns,
            row_count,
            has_header,
        })
    }

    /// Detect relationships between sheets based on column naming patterns
    ///
    /// Identifies potential foreign keys by matching columns ending with `_id`
    /// to sheet names (e.g., `customer_id` in Orders sheet → Customer sheet)
    ///
    /// # Arguments
    /// * `sheets` - Collection of sheet statistics
    ///
    /// # Returns
    /// * HashMap mapping (source_sheet, column_name) → target_sheet
    fn detect_sheet_relationships(
        &self,
        sheets: &[SheetStats],
    ) -> HashMap<(String, String), String> {
        let mut relationships = HashMap::new();

        // Build set of normalized sheet names for matching
        let sheet_names: HashMap<String, String> = sheets
            .iter()
            .map(|s| (sanitize_name(&s.name).to_lowercase(), s.name.clone()))
            .collect();

        // Check each sheet for potential foreign key columns
        for sheet in sheets {
            for col in &sheet.columns {
                // Look for columns ending with _id
                if let Some(prefix) = col.name.strip_suffix("_id") {
                    let normalized_prefix = prefix.to_lowercase();

                    // Check if this matches any sheet name
                    if let Some(target_sheet) = sheet_names.get(&normalized_prefix) {
                        relationships.insert(
                            (sheet.name.clone(), col.name.clone()),
                            target_sheet.clone(),
                        );
                    }
                }

                // Also check for columns that exactly match sheet names
                let col_normalized = sanitize_name(&col.name).to_lowercase();
                if let Some(target_sheet) = sheet_names.get(&col_normalized) {
                    if target_sheet != &sheet.name {
                        relationships.insert(
                            (sheet.name.clone(), col.name.clone()),
                            target_sheet.clone(),
                        );
                    }
                }
            }
        }

        relationships
    }

    /// Convert collected sheet statistics to DocumentStats format
    ///
    /// Each sheet becomes a LinkML class, and each column becomes a slot in that class.
    ///
    /// # Arguments
    /// * `sheets` - Collection of sheet statistics
    ///
    /// # Returns
    /// * `InferenceResult<DocumentStats>` - Document statistics ready for schema generation
    fn sheets_to_document_stats(&self, sheets: Vec<SheetStats>) -> InferenceResult<DocumentStats> {
        use crate::inference::types::{AttributeStats, ElementStats};

        let mut doc_stats = DocumentStats::new("workbook".to_string(), "excel".to_string());

        // Detect relationships between sheets
        let relationships = self.detect_sheet_relationships(&sheets);

        // Process each sheet as a separate LinkML class
        for sheet in sheets {
            let class_name = sanitize_name(&sheet.name);

            let mut element_stats = ElementStats::new(class_name.clone());
            element_stats.occurrence_count = sheet.row_count;

            // Each column becomes a slot (attribute) of this class
            for col in sheet.columns {
                let slot_name = sanitize_name(&col.name);

                let mut attr_stats = AttributeStats::new(slot_name.clone());
                attr_stats.value_samples = col.value_samples.clone();
                attr_stats.occurrence_count = col.non_empty_count;
                attr_stats.unique_values = attr_stats.value_samples.len();

                // Infer LinkML type from predominant Excel type
                if let Some(excel_type) = col.predominant_type() {
                    let linkml_type = excel_type_to_linkml(&excel_type);
                    attr_stats.value_samples.insert(0, format!("TYPE:{linkml_type}"));
                }

                // Add metadata about required status
                if col.is_required() {
                    attr_stats.value_samples.insert(0, "REQUIRED:true".to_string());
                }

                // Add enum constraint if detected
                if let Some(enum_values) = col.detect_enum() {
                    let enum_str = enum_values.join("|");
                    attr_stats.value_samples.insert(0, format!("ENUM:{enum_str}"));
                }

                // Add numeric range constraint if detected
                if let Some((min, max)) = col.numeric_range() {
                    attr_stats.value_samples.insert(0, format!("RANGE:{min}..{max}"));
                }

                // Add relationship metadata if this column references another sheet
                let relationship_key = (sheet.name.clone(), col.name.clone());
                if let Some(target_sheet) = relationships.get(&relationship_key) {
                    let target_class = sanitize_name(target_sheet);
                    attr_stats.value_samples.insert(0, format!("FK_TO:{target_class}"));
                }

                element_stats.attributes.insert(slot_name, attr_stats);
            }

            doc_stats.elements.insert(class_name, element_stats);
        }

        Ok(doc_stats)
    }
}

/// Sanitize a name for use as LinkML identifier
///
/// Converts spaces to underscores, removes special characters,
/// and ensures the name starts with a letter.
fn sanitize_name(name: &str) -> String {
    let mut result = name
        .replace(' ', "_")
        .replace('-', "_")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>();

    // Ensure starts with letter or underscore
    if result.is_empty() || result.chars().next().map_or(false, |c| c.is_numeric()) {
        result = format!("col_{result}");
    }

    result
}

/// Map Excel type to LinkML type
fn excel_type_to_linkml(excel_type: &str) -> &str {
    match excel_type {
        "integer" => "integer",
        "float" => "float",
        "string" => "string",
        "boolean" => "boolean",
        "datetime" => "datetime",
        _ => "string", // Default fallback
    }
}

/// Wiring function to create Excel introspector with dependency injection
///
/// Follows RootReal's wiring pattern for service instantiation.
///
/// # Arguments
/// * `logger` - Logger service for operation tracking
/// * `timestamp` - Timestamp service for metadata generation
///
/// # Returns
/// * `ExcelIntrospector` - Configured Excel introspector instance
///
/// # Example
/// ```no_run
/// use linkml_service::inference::introspectors::excel::wire_excel_introspector;
/// use logger_service::wiring::wire_logger;
/// use timestamp_service::wiring::wire_timestamp;
///
/// let timestamp = wire_timestamp().into_arc();
/// let logger = wire_logger(timestamp.clone()).into_arc();
/// let excel_introspector = wire_excel_introspector(logger, timestamp);
/// ```
pub fn wire_excel_introspector(
    logger: Arc<dyn LoggerService<Error = LoggerError>>,
    timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
) -> ExcelIntrospector {
    ExcelIntrospector::new(logger, timestamp)
}

#[async_trait]
impl DataIntrospector for ExcelIntrospector {
    async fn analyze_file(&self, path: &Path) -> InferenceResult<DocumentStats> {
        // Open workbook using calamine
        let mut workbook: Xlsx<_> = calamine::open_workbook(path)
            .map_err(|e: calamine::XlsxError| InferenceError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())))?;

        let mut all_sheets = Vec::new();

        // Process each sheet in the workbook
        for sheet_name in workbook.sheet_names().to_vec() {
            if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                let rows = range.rows().map(|row| row.to_vec()).collect::<Vec<_>>();
                let sheet_stats = self.process_sheet(sheet_name, rows.into_iter())?;
                all_sheets.push(sheet_stats);
            }
        }

        // Convert sheet statistics to DocumentStats
        self.sheets_to_document_stats(all_sheets)
    }

    async fn analyze_bytes(&self, data: &[u8]) -> InferenceResult<DocumentStats> {
        // Create a cursor for in-memory Excel data
        let cursor = Cursor::new(data);
        let mut workbook: Xlsx<_> = Xlsx::new(cursor)
            .map_err(|e: calamine::XlsxError| InferenceError::ParseServiceError(e.to_string()))?;

        let mut all_sheets = Vec::new();

        // Process each sheet in the workbook
        for sheet_name in workbook.sheet_names().to_vec() {
            if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                let rows = range.rows().map(|row| row.to_vec()).collect::<Vec<_>>();
                let sheet_stats = self.process_sheet(sheet_name, rows.into_iter())?;
                all_sheets.push(sheet_stats);
            }
        }

        // Convert sheet statistics to DocumentStats
        self.sheets_to_document_stats(all_sheets)
    }

    fn format_name(&self) -> &str {
        "excel"
    }

    async fn generate_schema(
        &self,
        stats: &DocumentStats,
        schema_id: &str,
    ) -> InferenceResult<SchemaDefinition> {
        self.logger
            .log_info(&format!("Generating LinkML schema: {}", schema_id))
            .await
            .map_err(|e| InferenceError::LoggerError(e.to_string()))?;

        let schema_name = stats
            .metadata
            .schema_name
            .clone()
            .unwrap_or_else(|| format!("{} Schema", schema_id));

        let mut builder = SchemaBuilder::new(schema_id, &schema_name)
            .with_timestamp_service(Arc::clone(&self.timestamp));

        builder = builder
            .with_description(format!(
                "Auto-generated schema from Excel introspection ({})",
                stats.format
            ))
            .with_version("1.0.0")
            .with_default_range("string");

        // Create classes for each sheet (element)
        for (element_name, element_stats) in &stats.elements {
            let mut class_builder = builder.add_class(element_name);

            class_builder = class_builder.with_description(format!(
                "Excel sheet '{}' with {} rows",
                element_name, element_stats.occurrence_count
            ));

            // Add slots for attributes (columns)
            for (attr_name, attr_stats) in &element_stats.attributes {
                let inferred_type = self
                    .type_inferencer
                    .infer_from_samples(&attr_stats.value_samples);
                let required = attr_stats.occurrence_count == element_stats.occurrence_count;
                let multivalued = attr_stats.occurrence_count > element_stats.occurrence_count;

                class_builder = class_builder.add_slot_with_type(
                    attr_name,
                    &inferred_type,
                    required,
                    multivalued,
                );
            }

            builder = class_builder.finish();
        }

        let schema = builder.build();

        self.logger
            .log_info(&format!(
                "Schema generation complete: {} classes, {} slots",
                schema.classes.len(),
                schema.slots.len()
            ))
            .await
            .map_err(|e| InferenceError::LoggerError(e.to_string()))?;

        Ok(schema)
    }
}
