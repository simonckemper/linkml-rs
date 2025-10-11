//! CSV data loader and dumper for `LinkML`
//!
//! This module provides functionality to load CSV/TSV files into `LinkML`
//! data instances and dump instances back to CSV/TSV format.

use async_trait::async_trait;
use csv::{ReaderBuilder, StringRecord, WriterBuilder};
use linkml_core::prelude::*;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::Path;

use super::traits::{
    DataDumper, DataInstance, DataLoader, DumpOptions, DumperError, DumperResult, LoadOptions,
    LoaderError, LoaderResult,
};

/// Options specific to CSV loading/dumping
#[derive(Debug, Clone)]
pub struct CsvOptions {
    /// Field delimiter (default: ',')
    pub delimiter: u8,

    /// Whether the first row contains headers
    pub has_headers: bool,

    /// Quote character (default: '"')
    pub quote: u8,

    /// Whether to use double quotes
    pub double_quote: bool,

    /// Comment character (lines starting with this are ignored)
    pub comment: Option<u8>,

    /// Whether to trim whitespace from fields
    pub trim: bool,

    /// Whether to use flexible parsing (variable field counts)
    pub flexible: bool,

    /// Encoding (currently only UTF-8 supported)
    pub encoding: String,
}

impl Default for CsvOptions {
    fn default() -> Self {
        Self {
            delimiter: b',',
            has_headers: true,
            quote: b'"',
            double_quote: true,
            comment: None,
            trim: true,
            flexible: false,
            encoding: "utf-8".to_string(),
        }
    }
}

impl CsvOptions {
    /// Create options for TSV format
    #[must_use]
    pub fn tsv() -> Self {
        Self {
            delimiter: b'\t',
            ..Default::default()
        }
    }
}

/// CSV data loader
pub struct CsvLoader {
    options: CsvOptions,
}

impl CsvLoader {
    /// Create a new CSV loader
    #[must_use]
    pub fn new() -> Self {
        Self {
            options: CsvOptions::default(),
        }
    }

    /// Create a new CSV loader with custom options
    #[must_use]
    pub fn with_options(options: CsvOptions) -> Self {
        Self { options }
    }

    /// Create a TSV loader
    #[must_use]
    pub fn tsv() -> Self {
        Self {
            options: CsvOptions::tsv(),
        }
    }

    /// Parse a CSV record into a data instance
    fn parse_record(
        &self,
        record: &StringRecord,
        headers: &[String],
        class_name: &str,
        schema: &SchemaDefinition,
        field_mappings: &HashMap<String, String>,
    ) -> LoaderResult<DataInstance> {
        let mut data = HashMap::new();
        let mut id = None;

        // Get class definition
        let _class_def = schema.classes.get(class_name).ok_or_else(|| {
            LoaderError::SchemaValidation(format!("Class '{class_name}' not found in schema"))
        })?;

        // Process each field
        for (i, value) in record.iter().enumerate() {
            if i >= headers.len() {
                if !self.options.flexible {
                    return Err(LoaderError::Parse(format!(
                        "Record has more fields than headers: {} > {}",
                        record.len(),
                        headers.len()
                    )));
                }
                continue;
            }

            let header = &headers[i];
            let field_name = field_mappings.get(header).unwrap_or(header);

            // Skip empty values
            if value.trim().is_empty() {
                continue;
            }

            // Check if this is an identifier field
            if let Some(slot_def) = schema.slots.get(field_name)
                && slot_def.identifier == Some(true)
            {
                id = Some(value.to_string());
            }

            // Convert value based on slot type
            let json_value = self.convert_value(value, field_name, schema)?;
            data.insert(field_name.clone(), json_value);
        }

        Ok(DataInstance {
            class_name: class_name.to_string(),
            data,
            id,
            metadata: HashMap::new(),
        })
    }

    /// Convert a string value to the appropriate `JSON` type
    fn convert_value(
        &self,
        value: &str,
        field_name: &str,
        schema: &SchemaDefinition,
    ) -> LoaderResult<JsonValue> {
        // Get slot definition to determine type
        if let Some(slot_def) = schema.slots.get(field_name)
            && let Some(range) = &slot_def.range
        {
            return self.convert_typed_value(value, range, slot_def);
        }

        // Default to string
        Ok(JsonValue::String(value.to_string()))
    }

    /// Convert value based on type
    fn convert_typed_value(
        &self,
        value: &str,
        type_name: &str,
        slot_def: &SlotDefinition,
    ) -> LoaderResult<JsonValue> {
        let trimmed = if self.options.trim {
            value.trim()
        } else {
            value
        };

        // Handle multivalued fields
        if slot_def.multivalued == Some(true) {
            // Split by common delimiters
            let values: Vec<&str> = if trimmed.contains(';') {
                trimmed.split(';').map(str::trim).collect()
            } else if trimmed.contains('|') {
                trimmed.split('|').map(str::trim).collect()
            } else if trimmed.contains(',') && !trimmed.contains('"') {
                trimmed.split(',').map(str::trim).collect()
            } else {
                vec![trimmed]
            };

            let json_values: std::result::Result<Vec<_>, _> = values
                .into_iter()
                .map(|v| Self::convert_single_value(v, type_name))
                .collect();

            return Ok(JsonValue::Array(json_values?));
        }

        Self::convert_single_value(trimmed, type_name)
    }

    /// Convert a single value
    fn convert_single_value(value: &str, type_name: &str) -> LoaderResult<JsonValue> {
        match type_name {
            "string" | "uri" | "uriorcurie" | "curie" | "ncname" => {
                Ok(JsonValue::String(value.to_string()))
            }

            "integer" => value
                .parse::<i64>()
                .map(|n| JsonValue::Number(n.into()))
                .map_err(|_| {
                    LoaderError::TypeConversion(format!("Cannot parse '{value}' as integer"))
                }),

            "float" | "double" | "decimal" => value
                .parse::<f64>()
                .map(|n| {
                    JsonValue::Number(serde_json::Number::from_f64(n).unwrap_or_else(|| 0.into()))
                })
                .map_err(|_| {
                    LoaderError::TypeConversion(format!("Cannot parse '{value}' as float"))
                }),

            "boolean" => match value.to_lowercase().as_str() {
                "true" | "yes" | "y" | "1" => Ok(JsonValue::Bool(true)),
                "false" | "no" | "n" | "0" => Ok(JsonValue::Bool(false)),
                _ => Err(LoaderError::TypeConversion(format!(
                    "Cannot parse '{value}' as boolean"
                ))),
            },

            "date" | "datetime" | "time" => {
                // For now, keep as string - could validate format
                Ok(JsonValue::String(value.to_string()))
            }

            _ => {
                // Check if it's an enum
                Ok(JsonValue::String(value.to_string()))
            }
        }
    }

    /// Infer the target class from headers and schema
    fn infer_target_class(
        &self,
        headers: &[String],
        schema: &SchemaDefinition,
    ) -> LoaderResult<String> {
        // Try to find a class that has most of the headers as slots
        let mut best_match = None;
        let mut best_score = 0;

        for (class_name, class_def) in &schema.classes {
            let mut score = 0;
            let all_slots = self.collect_all_slots(class_name, class_def, schema);

            for header in headers {
                if all_slots.contains(header) {
                    score += 1;
                }
            }

            if score > best_score {
                best_score = score;
                best_match = Some(class_name.clone());
            }
        }

        best_match.ok_or_else(|| {
            LoaderError::SchemaValidation(
                "Could not infer target class from CSV headers".to_string(),
            )
        })
    }

    /// Collect all slots including inherited ones
    fn collect_all_slots(
        &self,
        _class_name: &str,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> Vec<String> {
        let mut all_slots = Vec::new();

        // Add inherited slots
        if let Some(parent_name) = &class_def.is_a
            && let Some(parent_class) = schema.classes.get(parent_name)
        {
            let parent_slots = self.collect_all_slots(parent_name, parent_class, schema);
            all_slots.extend(parent_slots);
        }

        // Add direct slots
        all_slots.extend(class_def.slots.clone());

        // Add attributes
        all_slots.extend(class_def.attributes.keys().cloned());

        all_slots
    }
}

impl Default for CsvLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DataLoader for CsvLoader {
    fn name(&self) -> &str {
        if self.options.delimiter == b'\t' {
            "tsv"
        } else {
            "csv"
        }
    }

    fn description(&self) -> &'static str {
        "Loads data from CSV/TSV files"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        if self.options.delimiter == b'\t' {
            vec![".tsv", ".tab"]
        } else {
            vec![".csv"]
        }
    }

    async fn load_file(
        &self,
        path: &Path,
        schema: &SchemaDefinition,
        options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        let content = tokio::fs::read_to_string(path).await?;
        self.load_string(&content, schema, options).await
    }

    async fn load_string(
        &self,
        content: &str,
        schema: &SchemaDefinition,
        options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        let mut reader = ReaderBuilder::new()
            .delimiter(self.options.delimiter)
            .has_headers(self.options.has_headers)
            .quote(self.options.quote)
            .double_quote(self.options.double_quote)
            .comment(self.options.comment)
            .trim(csv::Trim::All)
            .flexible(self.options.flexible)
            .from_reader(content.as_bytes());

        // Get headers
        let headers: Vec<String> = if self.options.has_headers {
            reader
                .headers()
                .map_err(|e| LoaderError::Parse(format!("Failed to read headers: {e}")))?
                .iter()
                .map(std::string::ToString::to_string)
                .collect()
        } else {
            return Err(LoaderError::Configuration(
                "CSV without headers not yet supported".to_string(),
            ));
        };

        // Determine target class
        let target_class = if let Some(class) = &options.target_class {
            class.clone()
        } else if options.infer_types {
            self.infer_target_class(&headers, schema)?
        } else {
            return Err(LoaderError::Configuration(
                "No target class specified and type inference disabled".to_string(),
            ));
        };

        // Load records
        let mut instances = Vec::new();
        let mut error_count = 0;

        for (i, result) in reader.records().enumerate() {
            // Check limit
            if let Some(limit) = options.limit
                && instances.len() >= limit
            {
                break;
            }

            match result {
                Ok(record) => {
                    match self.parse_record(
                        &record,
                        &headers,
                        &target_class,
                        schema,
                        &options.field_mappings,
                    ) {
                        Ok(instance) => instances.push(instance),
                        Err(e) => {
                            if options.skip_invalid {
                                error_count += 1;
                                eprintln!("Warning: Skipping invalid record {}: {}", i + 1, e);
                            } else {
                                return Err(e);
                            }
                        }
                    }
                }
                Err(e) => {
                    if options.skip_invalid {
                        error_count += 1;
                        eprintln!("Warning: Skipping invalid record {}: {}", i + 1, e);
                    } else {
                        return Err(LoaderError::Parse(format!(
                            "Failed to read record {}: {}",
                            i + 1,
                            e
                        )));
                    }
                }
            }
        }

        if error_count > 0 {
            eprintln!("Total errors skipped: {error_count}");
        }

        Ok(instances)
    }

    async fn load_bytes(
        &self,
        data: &[u8],
        schema: &SchemaDefinition,
        options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        let content = String::from_utf8(data.to_vec())
            .map_err(|e| LoaderError::Parse(format!("Invalid UTF-8: {e}")))?;
        self.load_string(&content, schema, options).await
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> LoaderResult<()> {
        // Validate that schema is compatible with CSV loading

        // Check if schema has basic required elements
        if schema.name.is_empty() {
            return Err(LoaderError::SchemaValidation(
                "Schema name is required for CSV loading".to_string(),
            ));
        }

        // Validate that classes can be represented as CSV rows
        for (class_name, class_def) in &schema.classes {
            // CSV loading requires at least one slot or attribute to create meaningful rows
            if class_def.slots.is_empty() && class_def.attributes.is_empty() {
                return Err(LoaderError::SchemaValidation(format!(
                    "Class '{class_name}' has no slots or attributes - cannot represent as CSV row"
                )));
            }

            // Validate slots for CSV compatibility
            for slot_name in &class_def.slots {
                // Check if this slot exists in schema slots
                if !schema.slots.contains_key(slot_name) {
                    return Err(LoaderError::SchemaValidation(format!(
                        "Slot '{slot_name}' referenced in class '{class_name}' not found in schema slots"
                    )));
                }

                let slot_def = &schema.slots[slot_name];

                // Check if multivalued slots have proper serialization defined
                if slot_def.multivalued == Some(true) {
                    // Multivalued slots need special handling in CSV - warn about this
                    // but don't fail validation as they can be serialized as JSON or delimited strings
                }

                // Validate range constraints
                if let Some(range) = &slot_def.range
                    && !schema.classes.contains_key(range)
                    && !schema.enums.contains_key(range)
                    && !is_valid_csv_datatype(range)
                {
                    return Err(LoaderError::SchemaValidation(format!(
                        "Slot '{slot_name}' has invalid range '{range}' - not a valid class, enum, or CSV-compatible datatype"
                    )));
                }

                // Check domain constraints
                if let Some(domain) = &slot_def.domain
                    && !schema.classes.contains_key(domain)
                {
                    return Err(LoaderError::SchemaValidation(format!(
                        "Slot '{slot_name}' has invalid domain '{domain}' - class not found in schema"
                    )));
                }
            }

            // Validate attributes for CSV compatibility
            for (attr_name, _attr_def) in &class_def.attributes {
                // Check for CSV-problematic characters in attribute names (commas, quotes, newlines)
                if attr_name.contains([',', '"', '\n', '\r']) {
                    return Err(LoaderError::SchemaValidation(format!(
                        "Attribute name '{attr_name}' in class '{class_name}' contains CSV-incompatible characters (comma, quote, or newline)"
                    )));
                }
            }

            // Check inheritance chain for validity
            if let Some(parent_name) = &class_def.is_a
                && !schema.classes.contains_key(parent_name)
            {
                return Err(LoaderError::SchemaValidation(format!(
                    "Parent class '{parent_name}' for class '{class_name}' not found in schema"
                )));
            }
        }

        // Validate slot definitions
        for (slot_name, slot_def) in &schema.slots {
            // Check if range is valid
            if let Some(range) = &slot_def.range
                && !schema.classes.contains_key(range)
                && !schema.enums.contains_key(range)
                && !is_valid_csv_datatype(range)
            {
                return Err(LoaderError::SchemaValidation(format!(
                    "Slot '{slot_name}' has invalid range '{range}' - not a valid class, enum, or CSV-compatible datatype"
                )));
            }
        }

        // Validate enums for CSV compatibility
        for (enum_name, enum_def) in &schema.enums {
            // Check enum values for CSV compatibility
            for pv in &enum_def.permissible_values {
                let pv_text = match pv {
                    linkml_core::types::PermissibleValue::Simple(text)
                    | linkml_core::types::PermissibleValue::Complex { text, .. } => text,
                };

                if pv_text.is_empty() {
                    return Err(LoaderError::SchemaValidation(format!(
                        "Empty enum value in enum '{enum_name}'"
                    )));
                }

                // Check for CSV-problematic characters in enum values
                if pv_text.contains([',', '"', '\n', '\r']) {
                    return Err(LoaderError::SchemaValidation(format!(
                        "Enum value '{pv_text}' in enum '{enum_name}' contains CSV-incompatible characters (comma, quote, or newline)"
                    )));
                }
            }
        }

        Ok(())
    }
}

/// Check if a type name represents a valid CSV-compatible datatype
fn is_valid_csv_datatype(type_name: &str) -> bool {
    matches!(
        type_name,
        "string"
            | "boolean"
            | "integer"
            | "float"
            | "double"
            | "decimal"
            | "date"
            | "datetime"
            | "time"
            | "uri"
            | "uriorcurie"
            | "ncname"
            | "nodeidentifier"
            | "jsonpointer"
            | "jsonpath"
            | "sparqlpath"
            | "curie"
            | "int"
            | "long"
            | "short"
            | "byte"
            | "unsignedInt"
            | "unsignedLong"
            | "unsignedShort"
            | "unsignedByte"
            | "positiveInteger"
            | "nonNegativeInteger"
            | "negativeInteger"
            | "nonPositiveInteger"
    )
}

/// CSV data dumper
pub struct CsvDumper {
    options: CsvOptions,
}

impl CsvDumper {
    /// Create a new CSV dumper
    #[must_use]
    pub fn new() -> Self {
        Self {
            options: CsvOptions::default(),
        }
    }

    /// Create a new CSV dumper with custom options
    #[must_use]
    pub fn with_options(options: CsvOptions) -> Self {
        Self { options }
    }

    /// Create a TSV dumper
    #[must_use]
    pub fn tsv() -> Self {
        Self {
            options: CsvOptions::tsv(),
        }
    }

    /// Get headers for a class
    fn get_headers(
        &self,
        class_name: &str,
        schema: &SchemaDefinition,
        instances: &[DataInstance],
    ) -> Vec<String> {
        let mut headers = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // First, add slots from class definition in order
        if let Some(class_def) = schema.classes.get(class_name) {
            let all_slots = self.collect_all_slots(class_name, class_def, schema);

            for slot_name in &all_slots {
                if seen.insert(slot_name.clone()) {
                    headers.push(slot_name.clone());
                }
            }
        }

        // Then add any additional fields found in instances
        for instance in instances {
            for field in instance.data.keys() {
                if seen.insert(field.clone()) {
                    headers.push(field.clone());
                }
            }
        }

        headers
    }

    /// Convert `JSON` value to CSV string
    fn value_to_string(&self, value: &JsonValue) -> String {
        match value {
            JsonValue::Null => String::new(),
            JsonValue::Bool(b) => b.to_string(),
            JsonValue::Number(n) => n.to_string(),
            JsonValue::String(s) => s.clone(),
            JsonValue::Array(arr) => {
                // Join array values with semicolon
                arr.iter()
                    .map(|v| self.value_to_string(v))
                    .collect::<Vec<_>>()
                    .join(";")
            }
            JsonValue::Object(_) => {
                // Serialize as JSON for complex objects
                serde_json::to_string(value).unwrap_or_default()
            }
        }
    }

    /// Collect all slots including inherited ones
    fn collect_all_slots(
        &self,
        _class_name: &str,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> Vec<String> {
        let mut all_slots = Vec::new();

        // Add inherited slots
        if let Some(parent_name) = &class_def.is_a
            && let Some(parent_class) = schema.classes.get(parent_name)
        {
            let parent_slots = self.collect_all_slots(parent_name, parent_class, schema);
            all_slots.extend(parent_slots);
        }

        // Add direct slots
        all_slots.extend(class_def.slots.clone());

        // Add attributes
        all_slots.extend(class_def.attributes.keys().cloned());

        all_slots
    }
}

impl Default for CsvDumper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DataDumper for CsvDumper {
    fn name(&self) -> &str {
        if self.options.delimiter == b'\t' {
            "tsv"
        } else {
            "csv"
        }
    }

    fn description(&self) -> &'static str {
        "Dumps data to CSV/TSV format"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        if self.options.delimiter == b'\t' {
            vec![".tsv", ".tab"]
        } else {
            vec![".csv"]
        }
    }

    async fn dump_file(
        &self,
        instances: &[DataInstance],
        path: &Path,
        schema: &SchemaDefinition,
        options: &DumpOptions,
    ) -> DumperResult<()> {
        let content = self.dump_string(instances, schema, options).await?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    async fn dump_string(
        &self,
        instances: &[DataInstance],
        schema: &SchemaDefinition,
        options: &DumpOptions,
    ) -> DumperResult<String> {
        if instances.is_empty() {
            return Ok(String::new());
        }

        // Group instances by class
        let mut by_class: HashMap<String, Vec<&DataInstance>> = HashMap::new();
        for instance in instances {
            by_class
                .entry(instance.class_name.clone())
                .or_default()
                .push(instance);
        }

        // For now, only handle single class
        if by_class.len() > 1 {
            return Err(DumperError::Configuration(
                "CSV dumping of multiple classes not yet supported".to_string(),
            ));
        }

        let (class_name, class_instances) = by_class.into_iter().next().ok_or_else(|| {
            anyhow::anyhow!("by_class should have at least one entry after check")
        })?;

        // Apply limit if specified
        let instances_to_dump: Vec<&DataInstance> = if let Some(limit) = options.limit {
            class_instances.into_iter().take(limit).collect()
        } else {
            class_instances
        };

        // Get headers - convert Vec<&DataInstance> to Vec<DataInstance> for the method
        let instances_for_headers: Vec<DataInstance> =
            instances_to_dump.iter().map(|&i| i.clone()).collect();
        let mut headers = self.get_headers(&class_name, schema, &instances_for_headers);

        // Apply field mappings in reverse
        let reverse_mappings: HashMap<String, String> = options
            .field_mappings
            .iter()
            .map(|(k, v)| (v.clone(), k.clone()))
            .collect();

        headers = headers
            .into_iter()
            .map(|h| reverse_mappings.get(&h).unwrap_or(&h).clone())
            .collect();

        // Create CSV writer
        let mut wtr = WriterBuilder::new()
            .delimiter(self.options.delimiter)
            .quote(self.options.quote)
            .double_quote(self.options.double_quote)
            .from_writer(vec![]);

        // Write headers
        wtr.write_record(&headers).map_err(|e| {
            DumperError::Io(std::io::Error::other(format!(
                "Failed to write headers: {e}"
            )))
        })?;

        // Write records
        for instance in instances_to_dump {
            let mut record = Vec::new();

            for header in &headers {
                // Reverse map header to field name
                let field_name = options.field_mappings.get(header).unwrap_or(header);

                let value = if let Some(json_value) = instance.data.get(field_name) {
                    if json_value.is_null() && !options.include_nulls {
                        String::new()
                    } else {
                        self.value_to_string(json_value)
                    }
                } else {
                    String::new()
                };

                record.push(value);
            }

            wtr.write_record(&record).map_err(|e| {
                DumperError::Io(std::io::Error::other(format!(
                    "Failed to write record: {e}"
                )))
            })?;
        }

        // Get the written data
        let data = wtr.into_inner().map_err(|e| {
            DumperError::Io(std::io::Error::other(format!(
                "Failed to finish writing: {e}"
            )))
        })?;

        String::from_utf8(data)
            .map_err(|e| DumperError::Serialization(format!("Invalid UTF-8: {e}")))
    }

    async fn dump_bytes(
        &self,
        instances: &[DataInstance],
        schema: &SchemaDefinition,
        options: &DumpOptions,
    ) -> DumperResult<Vec<u8>> {
        let content = self.dump_string(instances, schema, options).await?;
        Ok(content.into_bytes())
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> DumperResult<()> {
        // Validate that schema is compatible with CSV dumping

        // Check if schema has basic required elements
        if schema.name.is_empty() {
            return Err(DumperError::SchemaValidation(
                "Schema name is required for CSV dumping".to_string(),
            ));
        }

        // Validate that classes can be dumped as CSV rows
        for (class_name, class_def) in &schema.classes {
            // CSV dumping requires at least one slot or attribute to create meaningful rows
            if class_def.slots.is_empty() && class_def.attributes.is_empty() {
                return Err(DumperError::SchemaValidation(format!(
                    "Class '{class_name}' has no slots or attributes - cannot dump as CSV row"
                )));
            }

            // Validate slots for CSV compatibility
            for slot_name in &class_def.slots {
                // Check if this slot exists in schema slots
                if !schema.slots.contains_key(slot_name) {
                    return Err(DumperError::SchemaValidation(format!(
                        "Slot '{slot_name}' referenced in class '{class_name}' not found in schema slots"
                    )));
                }

                let slot_def = &schema.slots[slot_name];

                // Validate range constraints
                if let Some(range) = &slot_def.range {
                    if !schema.classes.contains_key(range)
                        && !schema.enums.contains_key(range)
                        && !is_valid_csv_datatype(range)
                    {
                        return Err(DumperError::SchemaValidation(format!(
                            "Slot '{slot_name}' has invalid range '{range}' - not a valid class, enum, or CSV-compatible datatype"
                        )));
                    }
                }

                // Check domain constraints
                if let Some(domain) = &slot_def.domain
                    && !schema.classes.contains_key(domain)
                {
                    return Err(DumperError::SchemaValidation(format!(
                        "Slot '{slot_name}' has invalid domain '{domain}' - class not found in schema"
                    )));
                }
            }

            // Validate attributes for CSV compatibility
            for (attr_name, _attr_def) in &class_def.attributes {
                // Check for CSV-problematic characters in attribute names
                if attr_name.contains([',', '"', '\n', '\r']) {
                    return Err(DumperError::SchemaValidation(format!(
                        "Attribute name '{attr_name}' in class '{class_name}' contains CSV-incompatible characters (comma, quote, or newline)"
                    )));
                }
            }

            // Check inheritance chain for validity
            if let Some(parent_name) = &class_def.is_a
                && !schema.classes.contains_key(parent_name)
            {
                return Err(DumperError::SchemaValidation(format!(
                    "Parent class '{parent_name}' for class '{class_name}' not found in schema"
                )));
            }
        }

        // Validate enums for CSV compatibility
        for (enum_name, enum_def) in &schema.enums {
            // Check enum values for CSV compatibility
            for pv in &enum_def.permissible_values {
                let pv_text = match pv {
                    linkml_core::types::PermissibleValue::Simple(text)
                    | linkml_core::types::PermissibleValue::Complex { text, .. } => text,
                };

                if pv_text.is_empty() {
                    return Err(DumperError::SchemaValidation(format!(
                        "Empty enum value in enum '{enum_name}'"
                    )));
                }

                // Check for CSV-problematic characters in enum values
                if pv_text.contains([',', '"', '\n', '\r']) {
                    return Err(DumperError::SchemaValidation(format!(
                        "Enum value '{pv_text}' in enum '{enum_name}' contains CSV-incompatible characters (comma, quote, or newline)"
                    )));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};

    fn create_test_schema() -> SchemaDefinition {
        let mut schema = SchemaDefinition {
            name: "TestSchema".to_string(),
            ..Default::default()
        };

        // Person class
        let person_class = ClassDefinition {
            slots: vec![
                "id".to_string(),
                "name".to_string(),
                "age".to_string(),
                "email".to_string(),
                "tags".to_string(),
            ],
            ..Default::default()
        };
        schema.classes.insert("Person".to_string(), person_class);

        // Define slots
        let id_slot = SlotDefinition {
            identifier: Some(true),
            range: Some("string".to_string()),
            ..Default::default()
        };
        schema.slots.insert("id".to_string(), id_slot);

        let name_slot = SlotDefinition {
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        };
        schema.slots.insert("name".to_string(), name_slot);

        let age_slot = SlotDefinition {
            range: Some("integer".to_string()),
            ..Default::default()
        };
        schema.slots.insert("age".to_string(), age_slot);

        let email_slot = SlotDefinition {
            range: Some("string".to_string()),
            pattern: Some(r"^[^@]+@[^@]+\.[^@]+$".to_string()),
            ..Default::default()
        };
        schema.slots.insert("email".to_string(), email_slot);

        let tags_slot = SlotDefinition {
            range: Some("string".to_string()),
            multivalued: Some(true),
            ..Default::default()
        };
        schema.slots.insert("tags".to_string(), tags_slot);

        schema
    }

    #[tokio::test]
    async fn test_csv_load_and_dump() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let schema = create_test_schema();
        let loader = CsvLoader::new();
        let dumper = CsvDumper::new();

        let csv_content = r#"id,name,age,email,tags
p1,Alice,30,alice@example.com,"tag1;tag2"
p2,Bob,25,bob@example.com,tag3
"#;

        // Load CSV
        let options = LoadOptions {
            target_class: Some("Person".to_string()),
            validate: true,
            ..Default::default()
        };

        let instances = loader
            .load_string(csv_content, &schema, &options)
            .await
            .expect("should load CSV: {}");
        assert_eq!(instances.len(), 2);

        // Check first instance
        assert_eq!(instances[0].class_name, "Person");
        assert_eq!(instances[0].id, Some("p1".to_string()));
        assert_eq!(
            instances[0].data.get("name"),
            Some(&JsonValue::String("Alice".to_string()))
        );
        assert_eq!(
            instances[0].data.get("age"),
            Some(&JsonValue::Number(serde_json::Number::from(30)))
        );

        // Check multivalued field
        assert_eq!(
            instances[0].data.get("tags"),
            Some(&JsonValue::Array(vec![
                JsonValue::String("tag1".to_string()),
                JsonValue::String("tag2".to_string())
            ]))
        );

        // Dump back to CSV
        let dump_options = DumpOptions::default();
        let output = dumper
            .dump_string(&instances, &schema, &dump_options)
            .await
            .expect("should dump to CSV: {}");

        // Should contain the same data
        assert!(output.contains("Alice"));
        assert!(output.contains("Bob"));
        assert!(output.contains("tag1;tag2"));
        Ok(())
    }

    #[tokio::test]
    async fn test_tsv_format() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let schema = create_test_schema();
        let loader = CsvLoader::tsv();

        let tsv_content = "id\tname\tage\temail\ttags
p1\tAlice\t30\talice@example.com\ttag1;tag2
";

        let options = LoadOptions {
            target_class: Some("Person".to_string()),
            ..Default::default()
        };

        let instances = loader
            .load_string(tsv_content, &schema, &options)
            .await
            .expect("should load TSV: {}");
        assert_eq!(instances.len(), 1);
        assert_eq!(
            instances[0].data.get("name"),
            Some(&JsonValue::String("Alice".to_string()))
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_type_conversion() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let schema = create_test_schema();
        let loader = CsvLoader::new();

        let csv_content = r"id,name,age,email,tags
p1,Alice,30,alice@example.com,single
p2,Bob,not_a_number,bob@example.com,
";

        let options = LoadOptions {
            target_class: Some("Person".to_string()),
            skip_invalid: false,
            ..Default::default()
        };

        // Should fail on invalid integer
        let result = loader.load_string(csv_content, &schema, &options).await;
        assert!(result.is_err());

        // Should skip invalid with skip_invalid=true
        let options_skip = LoadOptions {
            target_class: Some("Person".to_string()),
            skip_invalid: true,
            ..Default::default()
        };

        let instances = loader
            .load_string(csv_content, &schema, &options_skip)
            .await
            .expect("should load with skip_invalid: {}");
        assert_eq!(instances.len(), 1); // Only valid record
        Ok(())
    }
}
