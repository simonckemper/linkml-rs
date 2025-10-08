//! Types and utilities for SchemaSheets format processing

use std::collections::HashMap;

/// Type of SchemaSheet row
#[derive(Debug, Clone, PartialEq)]
pub enum SchemaSheetType {
    /// Class definition row (has `>` prefix)
    ClassDefinition,
    /// Attribute/slot definition row (no `>` prefix, has field name)
    AttributeDefinition,
    /// Enum definition row
    EnumDefinition,
    /// Enum value row
    EnumValue,
    /// Type definition row
    TypeDefinition,
    /// Subset definition row
    SubsetDefinition,
    /// Empty or comment row
    Empty,
}

/// Parsed row from SchemaSheets format
#[derive(Debug, Clone)]
pub struct SchemaSheetRow {
    /// Row type
    pub row_type: SchemaSheetType,
    /// Class name (from `>` column)
    pub class_name: Option<String>,
    /// Field/attribute name
    pub field_name: Option<String>,
    /// Whether field is a key/identifier
    pub is_key: bool,
    /// Multiplicity/cardinality (e.g., "0..1", "1", "0..*")
    pub multiplicity: Option<String>,
    /// Range/type
    pub range: Option<String>,
    /// Description
    pub description: Option<String>,
    /// Parent class (is_a)
    pub is_a: Option<String>,
    /// Mixin classes
    pub mixins: Vec<String>,
    /// Whether field is required
    pub required: Option<bool>,
    /// Pattern constraint
    pub pattern: Option<String>,
    /// Minimum value
    pub minimum_value: Option<String>,
    /// Maximum value
    pub maximum_value: Option<String>,
    /// External mappings (e.g., schema.org, skos:exactMatch)
    pub mappings: HashMap<String, String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
    /// Element type (for detecting enums, types, subsets)
    pub element_type: Option<String>,
}

impl SchemaSheetRow {
    /// Create a new empty row
    pub fn new() -> Self {
        Self {
            row_type: SchemaSheetType::Empty,
            class_name: None,
            field_name: None,
            is_key: false,
            multiplicity: None,
            range: None,
            description: None,
            is_a: None,
            mixins: Vec::new(),
            required: None,
            pattern: None,
            minimum_value: None,
            maximum_value: None,
            mappings: HashMap::new(),
            metadata: HashMap::new(),
            element_type: None,
        }
    }

    /// Parse multiplicity into (min, max) cardinality
    ///
    /// Examples:
    /// - "1" -> (1, Some(1))
    /// - "0..1" -> (0, Some(1))
    /// - "0..*" -> (0, None)
    /// - "1..*" -> (1, None)
    pub fn parse_multiplicity(&self) -> Option<(u32, Option<u32>)> {
        let mult = self.multiplicity.as_ref()?;
        let mult = mult.trim();

        if mult.contains("..") {
            // Range format: "min..max" or "min..*"
            let parts: Vec<&str> = mult.split("..").collect();
            if parts.len() != 2 {
                return None;
            }

            let min = parts[0].trim().parse::<u32>().ok()?;
            let max = if parts[1].trim() == "*" {
                None
            } else {
                Some(parts[1].trim().parse::<u32>().ok()?)
            };

            Some((min, max))
        } else {
            // Single value: "1", "0", etc.
            let val = mult.parse::<u32>().ok()?;
            Some((val, Some(val)))
        }
    }

    /// Check if this field is multivalued (max cardinality > 1 or unbounded)
    pub fn is_multivalued(&self) -> bool {
        if let Some((_, max)) = self.parse_multiplicity() {
            match max {
                None => true,             // Unbounded (*)
                Some(m) if m > 1 => true, // Max > 1
                _ => false,
            }
        } else {
            false
        }
    }

    /// Check if this field is required (min cardinality >= 1)
    pub fn is_required_field(&self) -> bool {
        // Check explicit required flag first
        if let Some(req) = self.required {
            return req;
        }

        // Otherwise check multiplicity
        if let Some((min, _)) = self.parse_multiplicity() {
            min >= 1
        } else {
            false
        }
    }
}

impl Default for SchemaSheetRow {
    fn default() -> Self {
        Self::new()
    }
}

/// Column header mappings for SchemaSheets format
#[derive(Debug, Clone)]
pub struct ColumnMapping {
    /// Column index for class name (`>` column)
    pub class_col: Option<usize>,
    /// Column index for field name
    pub field_col: Option<usize>,
    /// Column index for key designation
    pub key_col: Option<usize>,
    /// Column index for multiplicity
    pub multiplicity_col: Option<usize>,
    /// Column index for range
    pub range_col: Option<usize>,
    /// Column index for description
    pub desc_col: Option<usize>,
    /// Column index for is_a (inheritance)
    pub is_a_col: Option<usize>,
    /// Column index for mixin
    pub mixin_col: Option<usize>,
    /// Column index for required flag
    pub required_col: Option<usize>,
    /// Column index for pattern
    pub pattern_col: Option<usize>,
    /// Column index for minimum_value
    pub min_value_col: Option<usize>,
    /// Column index for maximum_value
    pub max_value_col: Option<usize>,
    /// Column index for element type (enum, type, subset, class)
    pub element_type_col: Option<usize>,
    /// Mapping columns (e.g., "schema.org" -> column index)
    pub mapping_cols: HashMap<String, usize>,
}

impl ColumnMapping {
    /// Create a new column mapping by analyzing header row
    pub fn from_headers(headers: &[String]) -> Self {
        let mut mapping = Self {
            class_col: None,
            field_col: None,
            key_col: None,
            multiplicity_col: None,
            range_col: None,
            desc_col: None,
            is_a_col: None,
            mixin_col: None,
            required_col: None,
            pattern_col: None,
            min_value_col: None,
            max_value_col: None,
            element_type_col: None,
            mapping_cols: HashMap::new(),
        };

        for (idx, header) in headers.iter().enumerate() {
            let header_lower = header.trim().to_lowercase();

            match header_lower.as_str() {
                ">" | "class" => mapping.class_col = Some(idx),
                "field" | "slot" | "attribute" => mapping.field_col = Some(idx),
                "key" | "identifier" => mapping.key_col = Some(idx),
                "multiplicity" | "cardinality" => mapping.multiplicity_col = Some(idx),
                "range" | "type" => mapping.range_col = Some(idx),
                "desc" | "description" => mapping.desc_col = Some(idx),
                "is_a" | "parent" | "is a" => mapping.is_a_col = Some(idx),
                "mixin" | "mixins" => mapping.mixin_col = Some(idx),
                "required" => mapping.required_col = Some(idx),
                "pattern" | "regex" => mapping.pattern_col = Some(idx),
                "minimum_value" | "min_value" | "min" => mapping.min_value_col = Some(idx),
                "maximum_value" | "max_value" | "max" => mapping.max_value_col = Some(idx),
                "element_type" | "element type" | "metatype" => {
                    mapping.element_type_col = Some(idx)
                }
                _ => {
                    // Check if it's a mapping column (contains ":" or ends with "Match")
                    if header.contains(':') || header.ends_with("Match") || header.contains('.') {
                        mapping.mapping_cols.insert(header.clone(), idx);
                    }
                }
            }
        }

        mapping
    }

    /// Check if this looks like a SchemaSheets format header
    pub fn is_schemasheets_format(&self) -> bool {
        // Must have at least class and field columns
        self.class_col.is_some() && self.field_col.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_multiplicity() {
        let mut row = SchemaSheetRow::new();

        // Test single value
        row.multiplicity = Some("1".to_string());
        assert_eq!(row.parse_multiplicity(), Some((1, Some(1))));

        // Test range
        row.multiplicity = Some("0..1".to_string());
        assert_eq!(row.parse_multiplicity(), Some((0, Some(1))));

        // Test unbounded
        row.multiplicity = Some("0..*".to_string());
        assert_eq!(row.parse_multiplicity(), Some((0, None)));

        row.multiplicity = Some("1..*".to_string());
        assert_eq!(row.parse_multiplicity(), Some((1, None)));
    }

    #[test]
    fn test_is_multivalued() {
        let mut row = SchemaSheetRow::new();

        row.multiplicity = Some("1".to_string());
        assert!(!row.is_multivalued());

        row.multiplicity = Some("0..*".to_string());
        assert!(row.is_multivalued());

        row.multiplicity = Some("1..5".to_string());
        assert!(row.is_multivalued());
    }

    #[test]
    fn test_column_mapping_from_headers() {
        let headers = vec![
            ">".to_string(),
            "field".to_string(),
            "key".to_string(),
            "multiplicity".to_string(),
            "range".to_string(),
            "desc".to_string(),
            "schema.org".to_string(),
        ];

        let mapping = ColumnMapping::from_headers(&headers);

        assert_eq!(mapping.class_col, Some(0));
        assert_eq!(mapping.field_col, Some(1));
        assert_eq!(mapping.key_col, Some(2));
        assert_eq!(mapping.multiplicity_col, Some(3));
        assert_eq!(mapping.range_col, Some(4));
        assert_eq!(mapping.desc_col, Some(5));
        assert!(mapping.mapping_cols.contains_key("schema.org"));
        assert!(mapping.is_schemasheets_format());
    }
}
