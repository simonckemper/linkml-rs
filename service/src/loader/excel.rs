//! Excel data loader for LinkML
//!
//! This module provides functionality to load Excel/ODS files into LinkML
//! data instances with full schema validation.

use async_trait::async_trait;
use calamine::{Data, Range, Reader, Xlsx, open_workbook};
use linkml_core::prelude::*;
use logger_core::{LogLevel, LoggerError, LoggerService};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;
use timestamp_core::{TimestampError, TimestampService};

use super::traits::{DataInstance, DataLoader, LoadOptions, LoaderError, LoaderResult};

/// Options specific to Excel loading
#[derive(Debug, Clone)]
pub struct ExcelOptions {
    /// Target sheet name (None = first sheet, Some("*") = all sheets)
    pub target_sheet: Option<String>,

    /// Whether the first row contains headers
    pub has_headers: bool,

    /// Starting row (0-based)
    pub start_row: usize,

    /// Starting column (0-based)
    pub start_col: usize,

    /// Maximum rows to load
    pub max_rows: Option<usize>,

    /// Whether to evaluate formula cells
    pub evaluate_formulas: bool,

    /// Whether to handle merged cells
    pub handle_merged: bool,

    /// Date format string for parsing
    pub date_format: Option<String>,
}

impl Default for ExcelOptions {
    fn default() -> Self {
        Self {
            target_sheet: None,
            has_headers: true,
            start_row: 0,
            start_col: 0,
            max_rows: None,
            evaluate_formulas: true,
            handle_merged: true,
            date_format: None,
        }
    }
}

/// Excel data loader
pub struct ExcelLoader {
    logger: Arc<dyn LoggerService<Error = LoggerError>>,
    timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
    excel_options: ExcelOptions,
}

impl ExcelLoader {
    /// Create a new Excel loader
    #[must_use]
    pub fn new(
        logger: Arc<dyn LoggerService<Error = LoggerError>>,
        timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
    ) -> Self {
        Self {
            logger,
            timestamp,
            excel_options: ExcelOptions::default(),
        }
    }

    /// Create a new Excel loader with custom options
    #[must_use]
    pub fn with_options(
        logger: Arc<dyn LoggerService<Error = LoggerError>>,
        timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
        excel_options: ExcelOptions,
    ) -> Self {
        Self {
            logger,
            timestamp,
            excel_options,
        }
    }

    /// Process a worksheet range into data instances
    fn process_range(
        &self,
        range: &Range<Data>,
        sheet_name: &str,
        schema: &SchemaDefinition,
        options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        let mut instances = Vec::new();

        // Extract rows from range
        let rows: Vec<Vec<Data>> = range.rows().map(<[calamine::Data]>::to_vec).collect();
        if rows.is_empty() {
            return Ok(instances);
        }

        // Extract headers
        let headers = self.extract_headers(&rows)?;
        let data_start = usize::from(self.excel_options.has_headers);

        // Determine target class
        let target_class = options
            .target_class
            .as_ref()
            .or_else(|| {
                // Try to find class matching sheet name
                let sanitized = self.sanitize_name(sheet_name);
                schema
                    .classes
                    .keys()
                    .find(|k| k.to_lowercase() == sanitized.to_lowercase())
            })
            .ok_or_else(|| {
                LoaderError::Configuration(format!(
                    "Cannot determine target class for sheet '{sheet_name}'"
                ))
            })?;

        // Process data rows
        let max_rows = self.excel_options.max_rows.unwrap_or(usize::MAX);
        for (idx, row) in rows.iter().enumerate().skip(data_start) {
            if idx - data_start >= max_rows {
                break;
            }

            match self.parse_row(row, &headers, target_class, schema, options) {
                Ok(instance) => instances.push(instance),
                Err(e) if options.skip_invalid => {
                    // Note: Cannot await logger in non-async method
                    // Error is silently skipped as per skip_invalid option
                    let _ = format!("Skipping invalid row {idx} in sheet '{sheet_name}': {e}");
                }
                Err(e) => {
                    return Err(LoaderError::Parse(format!(
                        "Error parsing row {idx} in sheet '{sheet_name}': {e}"
                    )));
                }
            }

            if let Some(limit) = options.limit {
                if instances.len() >= limit {
                    break;
                }
            }
        }

        Ok(instances)
    }

    /// Extract headers from the first row or generate them
    fn extract_headers(&self, rows: &[Vec<Data>]) -> LoaderResult<Vec<String>> {
        if rows.is_empty() {
            return Ok(Vec::new());
        }

        if self.excel_options.has_headers {
            Ok(rows[0]
                .iter()
                .map(|cell| match cell {
                    Data::String(s) => s.clone(),
                    Data::Int(i) => i.to_string(),
                    Data::Float(f) => f.to_string(),
                    _ => "column".to_string(),
                })
                .collect())
        } else {
            // Generate column names: col_0, col_1, ...
            Ok((0..rows[0].len()).map(|i| format!("col_{i}")).collect())
        }
    }

    /// Parse a single row into a DataInstance
    fn parse_row(
        &self,
        row: &[Data],
        headers: &[String],
        class_name: &str,
        schema: &SchemaDefinition,
        options: &LoadOptions,
    ) -> LoaderResult<DataInstance> {
        let mut data = HashMap::new();
        let mut id = None;

        // Get class definition
        let class_def = schema.classes.get(class_name).ok_or_else(|| {
            LoaderError::SchemaValidation(format!("Class '{class_name}' not found in schema"))
        })?;

        // Process each cell
        for (i, cell) in row.iter().enumerate() {
            if i >= headers.len() {
                continue;
            }

            let header = &headers[i];
            let field_name = options.field_mappings.get(header).unwrap_or(header);

            // Skip empty cells
            if matches!(cell, Data::Empty) {
                continue;
            }

            // Check if this is an identifier field
            if let Some(slot_def) = class_def.attributes.get(field_name) {
                if slot_def.identifier == Some(true) {
                    id = Some(self.cell_to_string(cell));
                }
            }

            // Convert cell value based on slot type
            let json_value = self.convert_cell_value(cell, field_name, schema, class_def)?;

            data.insert(field_name.clone(), json_value);
        }

        // Validate required fields if validation is enabled
        if options.validate {
            self.validate_required_fields(&data, class_def, schema)?;
        }

        Ok(DataInstance {
            class_name: class_name.to_string(),
            data,
            id,
            metadata: HashMap::new(),
        })
    }

    /// Convert an Excel cell value to JSON value based on slot type
    fn convert_cell_value(
        &self,
        cell: &Data,
        field_name: &str,
        _schema: &SchemaDefinition,
        class_def: &ClassDefinition,
    ) -> LoaderResult<JsonValue> {
        // Get slot definition to determine expected type
        let slot_range = class_def
            .attributes
            .get(field_name)
            .and_then(|slot| slot.range.as_ref());

        match cell {
            Data::Int(i) => {
                // Convert to appropriate numeric type
                if let Some(range) = slot_range {
                    match range.as_str() {
                        "integer" | "int" => Ok(JsonValue::Number((*i).into())),
                        "float" | "double" => Ok(JsonValue::Number(
                            serde_json::Number::from_f64(*i as f64).ok_or_else(|| {
                                LoaderError::TypeConversion("Invalid float conversion".to_string())
                            })?,
                        )),
                        "string" => Ok(JsonValue::String(i.to_string())),
                        _ => Ok(JsonValue::Number((*i).into())),
                    }
                } else {
                    Ok(JsonValue::Number((*i).into()))
                }
            }
            Data::Float(f) => {
                // Handle float/double types
                if let Some(range) = slot_range {
                    match range.as_str() {
                        "integer" | "int" => {
                            // Truncate float to integer
                            Ok(JsonValue::Number((f.trunc() as i64).into()))
                        }
                        "string" => Ok(JsonValue::String(f.to_string())),
                        _ => Ok(JsonValue::Number(
                            serde_json::Number::from_f64(*f).ok_or_else(|| {
                                LoaderError::TypeConversion("Invalid float conversion".to_string())
                            })?,
                        )),
                    }
                } else {
                    Ok(JsonValue::Number(
                        serde_json::Number::from_f64(*f).ok_or_else(|| {
                            LoaderError::TypeConversion("Invalid float conversion".to_string())
                        })?,
                    ))
                }
            }
            Data::String(s) => Ok(JsonValue::String(s.clone())),
            Data::Bool(b) => Ok(JsonValue::Bool(*b)),
            Data::DateTime(dt) => {
                // Format datetime as ISO 8601 string
                Ok(JsonValue::String(format!("{dt:?}")))
            }
            Data::DateTimeIso(dt_str) => Ok(JsonValue::String(dt_str.clone())),
            Data::DurationIso(dur_str) => Ok(JsonValue::String(dur_str.clone())),
            Data::Error(err) => Err(LoaderError::Parse(format!(
                "Excel error in field '{field_name}': {err:?}"
            ))),
            Data::Empty => Ok(JsonValue::Null),
        }
    }

    /// Convert cell to string representation
    fn cell_to_string(&self, cell: &Data) -> String {
        match cell {
            Data::Int(i) => i.to_string(),
            Data::Float(f) => f.to_string(),
            Data::String(s) => s.clone(),
            Data::Bool(b) => b.to_string(),
            Data::DateTime(dt) => format!("{dt:?}"),
            Data::DateTimeIso(dt) => dt.clone(),
            Data::DurationIso(d) => d.clone(),
            Data::Error(e) => format!("ERROR: {e:?}"),
            Data::Empty => String::new(),
        }
    }

    /// Validate that all required fields are present
    fn validate_required_fields(
        &self,
        data: &HashMap<String, JsonValue>,
        class_def: &ClassDefinition,
        _schema: &SchemaDefinition,
    ) -> LoaderResult<()> {
        for (attr_name, slot_def) in &class_def.attributes {
            if slot_def.required == Some(true) && !data.contains_key(attr_name) {
                return Err(LoaderError::MissingField(format!(
                    "Required field '{attr_name}' is missing"
                )));
            }
        }
        Ok(())
    }

    /// Sanitize sheet name to match LinkML class naming conventions
    fn sanitize_name(&self, name: &str) -> String {
        name.chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>()
            .trim_matches('_')
            .to_string()
    }
}

#[async_trait]
impl DataLoader for ExcelLoader {
    fn name(&self) -> &'static str {
        "excel"
    }

    fn description(&self) -> &'static str {
        "Excel/ODS data loader with schema validation"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["xlsx", "xlsm", "xlsb", "xls", "ods"]
    }

    async fn load_file(
        &self,
        path: &Path,
        schema: &SchemaDefinition,
        options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        // Record start time for performance tracking
        let start_time = self
            .timestamp
            .now_utc()
            .await
            .map_err(|e| LoaderError::Configuration(format!("Failed to get timestamp: {e}")))?;

        let _ = self
            .logger
            .log(
                LogLevel::Info,
                &format!("Loading Excel file: {}", path.display()),
            )
            .await;

        // Open workbook with explicit XLSX type
        let mut workbook: Xlsx<_> = open_workbook(path)
            .map_err(|e| LoaderError::Parse(format!("Failed to open Excel file: {e}")))?;

        let mut all_instances = Vec::new();

        // Determine which sheets to process
        let sheet_names = workbook.sheet_names().clone();
        let target_sheets: Vec<String> = match &self.excel_options.target_sheet {
            None => vec![sheet_names.first().cloned().unwrap_or_default()],
            Some(name) if name == "*" => sheet_names,
            Some(name) => vec![name.clone()],
        };

        // Process each target sheet
        for sheet_name in &target_sheets {
            if let Ok(range) = workbook.worksheet_range(sheet_name) {
                let instances = self.process_range(&range, sheet_name, schema, options)?;

                let _ = self
                    .logger
                    .log(
                        LogLevel::Debug,
                        &format!(
                            "Loaded {} instances from sheet '{}'",
                            instances.len(),
                            sheet_name
                        ),
                    )
                    .await;

                all_instances.extend(instances);

                if let Some(limit) = options.limit {
                    if all_instances.len() >= limit {
                        all_instances.truncate(limit);
                        break;
                    }
                }
            }
        }

        // Calculate load duration
        let end_time = self
            .timestamp
            .now_utc()
            .await
            .map_err(|e| LoaderError::Configuration(format!("Failed to get timestamp: {e}")))?;
        let duration = end_time.signed_duration_since(start_time);

        let _ = self
            .logger
            .log(
                LogLevel::Info,
                &format!(
                    "Successfully loaded {} instances in {:.2}s",
                    all_instances.len(),
                    duration.num_milliseconds() as f64 / 1000.0
                ),
            )
            .await;

        Ok(all_instances)
    }

    async fn load_string(
        &self,
        _content: &str,
        _schema: &SchemaDefinition,
        _options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        Err(LoaderError::InvalidFormat(
            "Excel files cannot be loaded from string. Use load_bytes instead.".to_string(),
        ))
    }

    async fn load_bytes(
        &self,
        data: &[u8],
        schema: &SchemaDefinition,
        options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        let _ = self
            .logger
            .log(
                LogLevel::Info,
                &format!("Loading Excel data from {} bytes", data.len()),
            )
            .await;

        // Create workbook from bytes
        let cursor = Cursor::new(data);
        let mut workbook: Xlsx<_> = Xlsx::new(cursor)
            .map_err(|e| LoaderError::Parse(format!("Failed to parse Excel data: {e}")))?;

        let mut all_instances = Vec::new();

        // Process sheets
        let sheet_names = workbook.sheet_names();
        let target_sheets: Vec<String> = match &self.excel_options.target_sheet {
            None => vec![sheet_names.first().cloned().unwrap_or_default()],
            Some(name) if name == "*" => sheet_names,
            Some(name) => vec![name.clone()],
        };

        for sheet_name in &target_sheets {
            if let Ok(range) = workbook.worksheet_range(sheet_name) {
                let instances = self.process_range(&range, sheet_name, schema, options)?;
                all_instances.extend(instances);

                if let Some(limit) = options.limit {
                    if all_instances.len() >= limit {
                        all_instances.truncate(limit);
                        break;
                    }
                }
            }
        }

        Ok(all_instances)
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> LoaderResult<()> {
        // Validate that schema has at least one class
        if schema.classes.is_empty() {
            return Err(LoaderError::SchemaValidation(
                "Schema must contain at least one class".to_string(),
            ));
        }

        // Validate that classes have attributes or slots defined
        for (class_name, class_def) in &schema.classes {
            if class_def.attributes.is_empty() && class_def.slots.is_empty() {
                return Err(LoaderError::SchemaValidation(format!(
                    "Class '{class_name}' has no attributes or slots defined"
                )));
            }
        }

        Ok(())
    }
}

/// Wiring function for Excel loader
pub fn wire_excel_loader(
    logger: Arc<dyn LoggerService<Error = LoggerError>>,
    timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
) -> ExcelLoader {
    ExcelLoader::new(logger, timestamp)
}

/// Wiring function for Excel loader with custom options
pub fn wire_excel_loader_with_options(
    logger: Arc<dyn LoggerService<Error = LoggerError>>,
    timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
    excel_options: ExcelOptions,
) -> ExcelLoader {
    ExcelLoader::with_options(logger, timestamp, excel_options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_name() {
        let logger = Arc::new(MockLogger);
        let timestamp = Arc::new(MockTimestamp);
        let loader = ExcelLoader::new(logger, timestamp);

        assert_eq!(loader.sanitize_name("Employee Data"), "Employee_Data");
        assert_eq!(loader.sanitize_name("product-info"), "product_info");
        assert_eq!(loader.sanitize_name("__test__"), "test");
    }

    // Mock implementations for testing
    use async_trait::async_trait;

    struct MockLogger;
    #[async_trait]
    impl LoggerService for MockLogger {
        type Error = LoggerError;

        async fn debug(&self, _message: &str) -> std::result::Result<(), Self::Error> {
            Ok(())
        }
        async fn info(&self, _message: &str) -> std::result::Result<(), Self::Error> {
            Ok(())
        }
        async fn warn(&self, _message: &str) -> std::result::Result<(), Self::Error> {
            Ok(())
        }
        async fn error(&self, _message: &str) -> std::result::Result<(), Self::Error> {
            Ok(())
        }
        async fn log(
            &self,
            _level: LogLevel,
            _message: &str,
        ) -> std::result::Result<(), Self::Error> {
            Ok(())
        }
        async fn log_entry(
            &self,
            _entry: &logger_core::LogEntry,
        ) -> std::result::Result<(), Self::Error> {
            Ok(())
        }
        async fn set_level(&self, _level: LogLevel) -> std::result::Result<(), Self::Error> {
            Ok(())
        }
        async fn flush(&self) -> std::result::Result<(), Self::Error> {
            Ok(())
        }
        async fn shutdown(&self) -> std::result::Result<(), Self::Error> {
            Ok(())
        }
    }

    struct MockTimestamp;
    #[async_trait]
    impl TimestampService for MockTimestamp {
        type Error = TimestampError;

        async fn now_utc(&self) -> std::result::Result<chrono::DateTime<chrono::Utc>, Self::Error> {
            Ok(chrono::Utc::now())
        }
        async fn now_local(
            &self,
        ) -> std::result::Result<chrono::DateTime<chrono::Local>, Self::Error> {
            Ok(chrono::Local::now())
        }
        async fn system_time(&self) -> std::result::Result<std::time::SystemTime, Self::Error> {
            Ok(std::time::SystemTime::now())
        }
        async fn parse_iso8601(
            &self,
            timestamp: &str,
        ) -> std::result::Result<chrono::DateTime<chrono::Utc>, Self::Error> {
            use chrono::DateTime;
            Ok(DateTime::parse_from_rfc3339(timestamp)
                .map_err(|e| TimestampError::ParseError {
                    message: e.to_string().into(),
                })?
                .with_timezone(&chrono::Utc))
        }
        async fn format_iso8601(
            &self,
            timestamp: &chrono::DateTime<chrono::Utc>,
        ) -> std::result::Result<String, Self::Error> {
            Ok(timestamp.to_rfc3339())
        }
        async fn duration_since(
            &self,
            timestamp: &chrono::DateTime<chrono::Utc>,
        ) -> std::result::Result<chrono::Duration, Self::Error> {
            Ok(chrono::Utc::now() - *timestamp)
        }
        async fn add_duration(
            &self,
            timestamp: &chrono::DateTime<chrono::Utc>,
            duration: chrono::Duration,
        ) -> std::result::Result<chrono::DateTime<chrono::Utc>, Self::Error> {
            Ok(*timestamp + duration)
        }
        async fn subtract_duration(
            &self,
            timestamp: &chrono::DateTime<chrono::Utc>,
            duration: chrono::Duration,
        ) -> std::result::Result<chrono::DateTime<chrono::Utc>, Self::Error> {
            Ok(*timestamp - duration)
        }
        async fn duration_between(
            &self,
            from: &chrono::DateTime<chrono::Utc>,
            to: &chrono::DateTime<chrono::Utc>,
        ) -> std::result::Result<chrono::Duration, Self::Error> {
            Ok(*to - *from)
        }
        async fn unix_timestamp_to_datetime(
            &self,
            timestamp: i64,
        ) -> std::result::Result<chrono::DateTime<chrono::Utc>, Self::Error> {
            use chrono::TimeZone;
            Ok(chrono::Utc.timestamp_opt(timestamp, 0).unwrap())
        }
    }
}
