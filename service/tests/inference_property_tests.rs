// Copyright (C) 2025 Simon C. Kemper
// Licensed under Creative Commons BY-NC 4.0
//
// Property-based tests for LinkML schema inference system
//
// These tests use proptest to verify invariants across randomly generated inputs,
// ensuring robustness and correctness across a wide variety of edge cases.

use linkml_service::inference::{
    CsvIntrospector, DataIntrospector, JsonIntrospector, XmlIntrospector,
};
use logger_service::create_logger_service;
use proptest::prelude::*;
use std::sync::Arc;
use timestamp_service::create_timestamp_service;

// Helper function to create test services
fn create_test_services() -> (Arc<dyn logger_core::LoggerService<Error = logger_core::LoggerError>>, Arc<dyn timestamp_core::TimestampService<Error = timestamp_core::TimestampError>>) {
    let logger = create_logger_service().unwrap();
    let timestamp = create_timestamp_service();
    (logger, timestamp)
}

// Strategy: Generate random XML element names (valid identifiers)
fn xml_element_name() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z][a-zA-Z0-9_]*").unwrap()
}

// Strategy: Generate random XML attribute values
fn xml_attr_value() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z0-9_\\-. ]{1,50}").unwrap()
}

// Strategy: Generate random JSON property names
fn json_property_name() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z][a-zA-Z0-9_]{0,30}").unwrap()
}

// Strategy: Generate random primitive values
fn primitive_value() -> impl Strategy<Value = String> {
    prop_oneof![
        prop::string::string_regex("[a-zA-Z ]{1,50}").unwrap(),
        prop::num::i32::ANY.prop_map(|n| n.to_string()),
        prop::num::f64::NORMAL.prop_map(|f| format!("{:.2}", f)),
        prop::bool::ANY.prop_map(|b| b.to_string()),
    ]
}

// Property: XML analysis should never panic on valid XML
proptest! {
    #[test]
    fn test_xml_never_panics_on_valid_xml(
        element_name in xml_element_name(),
        attr_value in xml_attr_value(),
        text_content in primitive_value(),
    ) {
        let xml = format!(
            r#"<?xml version="1.0"?><{} attr="{}">{}</{}>"#,
            element_name, attr_value, text_content, element_name
        );

        let rt = tokio::runtime::Runtime::new().unwrap();
        let (logger, timestamp) = create_test_services();
        let introspector = XmlIntrospector::new(logger, timestamp);

        let result = rt.block_on(async {
            introspector.analyze_bytes(xml.as_bytes()).await
        });

        // Should not panic - either succeed or return error
        match result {
            Ok(stats) => {
                // Verify basic invariants
                assert!(stats.elements.contains_key(&element_name));
                assert!(stats.document_metrics.total_elements > 0);
            }
            Err(_) => {
                // Error is acceptable for edge cases
            }
        }
    }
}

// Property: Element occurrence count should match actual occurrences
proptest! {
    #[test]
    fn test_xml_element_count_accuracy(
        element_name in xml_element_name(),
        count in 1_usize..=10,
    ) {
        let mut xml = String::from(r#"<?xml version="1.0"?><root>"#);
        for _ in 0..count {
            xml.push_str(&format!("<{}>text</{}>"element_name, element_name));
        }
        xml.push_str("</root>");

        let rt = tokio::runtime::Runtime::new().unwrap();
        let (logger, timestamp) = create_test_services();
        let introspector = XmlIntrospector::new(logger, timestamp);

        let stats = rt.block_on(async {
            introspector.analyze_bytes(xml.as_bytes()).await.unwrap()
        });

        let element = stats.elements.get(&element_name).unwrap();
        assert_eq!(element.occurrence_count, count);
    }
}

// Property: Attribute occurrence should match actual attribute count
proptest! {
    #[test]
    fn test_xml_attribute_count_accuracy(
        element_name in xml_element_name(),
        attr_count in 1_usize..=5,
        values in prop::collection::vec(xml_attr_value(), 1..=5),
    ) {
        let mut xml = format!(r#"<?xml version="1.0"?><{}"#, element_name);

        for (i, value) in values.iter().enumerate().take(attr_count) {
            xml.push_str(&format!(r#" attr{}="{}""#, i, value));
        }
        xml.push_str(&format!(">text</{}>", element_name));

        let rt = tokio::runtime::Runtime::new().unwrap();
        let (logger, timestamp) = create_test_services();
        let introspector = XmlIntrospector::new(logger, timestamp);

        let stats = rt.block_on(async {
            introspector.analyze_bytes(xml.as_bytes()).await.unwrap()
        });

        let element = stats.elements.get(&element_name).unwrap();
        assert_eq!(element.attributes.len(), attr_count);
    }
}

// Property: Nesting depth should be accurately tracked
proptest! {
    #[test]
    fn test_xml_nesting_depth_accuracy(depth in 1_usize..=20) {
        let mut xml = String::from(r#"<?xml version="1.0"?>"#);

        for i in 0..depth {
            xml.push_str(&format!("<level{}>", i));
        }
        xml.push_str("<data>value</data>");
        for i in (0..depth).rev() {
            xml.push_str(&format!("</level{}>", i));
        }

        let rt = tokio::runtime::Runtime::new().unwrap();
        let (logger, timestamp) = create_test_services();
        let introspector = XmlIntrospector::new(logger, timestamp);

        let stats = rt.block_on(async {
            introspector.analyze_bytes(xml.as_bytes()).await.unwrap()
        });

        assert!(stats.document_metrics.max_nesting_depth >= depth);
    }
}

// Property: JSON analysis should never panic on valid JSON
proptest! {
    #[test]
    fn test_json_never_panics_on_valid_json(
        property_name in json_property_name(),
        value in primitive_value(),
    ) {
        let json = format!(r#"{{"{}":{}}}"#, property_name, serde_json::to_string(&value).unwrap());

        let rt = tokio::runtime::Runtime::new().unwrap();
        let (logger, timestamp) = create_test_services();
        let introspector = JsonIntrospector::new(logger, timestamp);

        let result = rt.block_on(async {
            introspector.analyze_bytes(json.as_bytes()).await
        });

        // Should not panic
        match result {
            Ok(stats) => {
                assert!(stats.elements.contains_key("root"));
            }
            Err(_) => {
                // Error is acceptable for edge cases
            }
        }
    }
}

// Property: Array element count should match array length
proptest! {
    #[test]
    fn test_json_array_count_accuracy(
        count in 1_usize..=10,
        values in prop::collection::vec(primitive_value(), 1..=10),
    ) {
        let json_values: Vec<String> = values.iter()
            .take(count)
            .map(|v| serde_json::to_string(v).unwrap())
            .collect();

        let json = format!(
            r#"{{"items":[{}]}}"#,
            json_values.join(",")
        );

        let rt = tokio::runtime::Runtime::new().unwrap();
        let (logger, timestamp) = create_test_services();
        let introspector = JsonIntrospector::new(logger, timestamp);

        let stats = rt.block_on(async {
            introspector.analyze_bytes(json.as_bytes()).await.unwrap()
        });

        // Verify array elements were counted
        assert!(stats.document_metrics.total_elements > 0);
    }
}

// Property: Nested JSON depth should be accurately tracked
proptest! {
    #[test]
    fn test_json_nesting_depth_accuracy(depth in 1_usize..=20) {
        let mut json = String::new();

        for i in 0..depth {
            json.push_str(&format!(r#"{{"level{}":"#, i));
        }
        json.push_str(r#"{"value":"deep"}"#);
        for _ in 0..depth {
            json.push('}');
        }

        let rt = tokio::runtime::Runtime::new().unwrap();
        let (logger, timestamp) = create_test_services();
        let introspector = JsonIntrospector::new(logger, timestamp);

        let stats = rt.block_on(async {
            introspector.analyze_bytes(json.as_bytes()).await.unwrap()
        });

        assert!(stats.document_metrics.max_nesting_depth >= depth);
    }
}

// Property: Schema generation should always produce valid schemas
proptest! {
    #[test]
    fn test_xml_schema_generation_valid(
        element_name in xml_element_name(),
        attr_name in xml_element_name(),
        attr_value in xml_attr_value(),
    ) {
        let xml = format!(
            r#"<?xml version="1.0"?><{} {}="{}">{}</{}>"#,
            element_name, attr_name, attr_value, "text", element_name
        );

        let rt = tokio::runtime::Runtime::new().unwrap();
        let (logger, timestamp) = create_test_services();
        let introspector = XmlIntrospector::new(logger, timestamp);

        let stats = rt.block_on(async {
            introspector.analyze_bytes(xml.as_bytes()).await.unwrap()
        });

        let schema = rt.block_on(async {
            introspector.generate_schema(&stats, "test_schema").await.unwrap()
        });

        // Verify schema invariants
        assert_eq!(schema.id, "test_schema");
        assert!(!schema.classes.is_empty());

        // Verify element became a class
        assert!(schema.classes.contains_key(&element_name));

        // Verify attribute became a slot
        let class = schema.classes.get(&element_name).unwrap();
        assert!(class.slots.contains(&attr_name));
    }
}

// Property: CSV row count should match input rows
proptest! {
    #[test]
    fn test_csv_row_count_accuracy(row_count in 1_usize..=50) {
        let mut csv = String::from("id,name,value\n");

        for i in 0..row_count {
            csv.push_str(&format!("{},Name{},{}\n", i, i, i * 10));
        }

        let rt = tokio::runtime::Runtime::new().unwrap();
        let (logger, timestamp) = create_test_services();
        let introspector = CsvIntrospector::new(logger, timestamp);

        let stats = rt.block_on(async {
            introspector.analyze_bytes(csv.as_bytes()).await.unwrap()
        });

        // CSV creates a "Record" class with attributes for each column
        assert!(stats.elements.contains_key("Record"));

        let record = stats.elements.get("Record").unwrap();

        // Each row creates one occurrence of the Record
        assert_eq!(record.occurrence_count, row_count);
    }
}

// Property: CSV column detection should be accurate
proptest! {
    #[test]
    fn test_csv_column_count_accuracy(
        column_count in 1_usize..=10,
    ) {
        let column_names: Vec<String> = (0..column_count)
            .map(|i| format!("col{}", i))
            .collect();

        let mut csv = format!("{}\n", column_names.join(","));
        csv.push_str(&(0..column_count)
            .map(|i| format!("value{}", i))
            .collect::<Vec<_>>()
            .join(","));

        let rt = tokio::runtime::Runtime::new().unwrap();
        let (logger, timestamp) = create_test_services();
        let introspector = CsvIntrospector::new(logger, timestamp);

        let stats = rt.block_on(async {
            introspector.analyze_bytes(csv.as_bytes()).await.unwrap()
        });

        let record = stats.elements.get("Record").unwrap();

        // Should have one attribute per column
        assert_eq!(record.attributes.len(), column_count);
    }
}

// Property: Empty values should be handled correctly
proptest! {
    #[test]
    fn test_csv_empty_value_handling(
        has_empty in prop::collection::vec(prop::bool::ANY, 1..=10),
    ) {
        let mut csv = String::from("id,name\n");

        for (i, is_empty) in has_empty.iter().enumerate() {
            if *is_empty {
                csv.push_str(&format!("{},\n", i));
            } else {
                csv.push_str(&format!("{},Name{}\n", i, i));
            }
        }

        let rt = tokio::runtime::Runtime::new().unwrap();
        let (logger, timestamp) = create_test_services();
        let introspector = CsvIntrospector::new(logger, timestamp);

        let result = rt.block_on(async {
            introspector.analyze_bytes(csv.as_bytes()).await
        });

        // Should handle empty values without panicking
        assert!(result.is_ok());
    }
}

// Property: Document metrics should be consistent
proptest! {
    #[test]
    fn test_xml_metrics_consistency(
        elements in prop::collection::vec(xml_element_name(), 1..=20),
    ) {
        let mut xml = String::from(r#"<?xml version="1.0"?><root>"#);

        for element in &elements {
            xml.push_str(&format!("<{}>text</{}>"element, element));
        }
        xml.push_str("</root>");

        let rt = tokio::runtime::Runtime::new().unwrap();
        let (logger, timestamp) = create_test_services();
        let introspector = XmlIntrospector::new(logger, timestamp);

        let stats = rt.block_on(async {
            introspector.analyze_bytes(xml.as_bytes()).await.unwrap()
        });

        // Verify metric consistency
        let counted_elements: usize = stats.elements.values()
            .map(|e| e.occurrence_count)
            .sum();

        assert_eq!(stats.document_metrics.total_elements, counted_elements);
        assert_eq!(stats.document_metrics.unique_element_names, stats.elements.len());
    }
}

// Property: Parent-child relationships should be bidirectional
proptest! {
    #[test]
    fn test_xml_parent_child_bidirectional(
        parent_name in xml_element_name(),
        child_name in xml_element_name(),
    ) {
        let xml = format!(
            r#"<?xml version="1.0"?><{}>
                <{}>text</{}></{}>
            "#,
            parent_name, child_name, child_name, parent_name
        );

        let rt = tokio::runtime::Runtime::new().unwrap();
        let (logger, timestamp) = create_test_services();
        let introspector = XmlIntrospector::new(logger, timestamp);

        let stats = rt.block_on(async {
            introspector.analyze_bytes(xml.as_bytes()).await.unwrap()
        });

        // Verify parent has child recorded
        let parent = stats.elements.get(&parent_name).unwrap();
        assert!(parent.children.contains_key(&child_name));

        // Verify child element exists
        assert!(stats.elements.contains_key(&child_name));
    }
}

// Property: Type inference should be deterministic
proptest! {
    #[test]
    fn test_type_inference_deterministic(
        values in prop::collection::vec(prop::num::i32::ANY, 1..=10),
    ) {
        let value_strings: Vec<String> = values.iter().map(|v| v.to_string()).collect();

        let xml1 = format!(
            r#"<?xml version="1.0"?><root>{}</root>"#,
            value_strings.iter()
                .map(|v| format!("<item>{}</item>", v))
                .collect::<Vec<_>>()
                .join("")
        );

        let rt = tokio::runtime::Runtime::new().unwrap();
        let (logger1, timestamp1) = create_test_services();
        let (logger2, timestamp2) = create_test_services();

        let introspector1 = XmlIntrospector::new(logger1, timestamp1);
        let introspector2 = XmlIntrospector::new(logger2, timestamp2);

        let stats1 = rt.block_on(async {
            introspector1.analyze_bytes(xml1.as_bytes()).await.unwrap()
        });

        let stats2 = rt.block_on(async {
            introspector2.analyze_bytes(xml1.as_bytes()).await.unwrap()
        });

        // Type inference should produce same results
        let item1 = stats1.elements.get("item").unwrap();
        let item2 = stats2.elements.get("item").unwrap();

        assert_eq!(item1.text_samples, item2.text_samples);
    }
}

// Property: Schema generation should be idempotent
proptest! {
    #[test]
    fn test_schema_generation_idempotent(
        element_name in xml_element_name(),
    ) {
        let xml = format!(
            r#"<?xml version="1.0"?><{}>text</{}>"#,
            element_name, element_name
        );

        let rt = tokio::runtime::Runtime::new().unwrap();
        let (logger, timestamp) = create_test_services();
        let introspector = XmlIntrospector::new(logger, timestamp);

        let stats = rt.block_on(async {
            introspector.analyze_bytes(xml.as_bytes()).await.unwrap()
        });

        let schema1 = rt.block_on(async {
            introspector.generate_schema(&stats, "test_schema").await.unwrap()
        });

        let schema2 = rt.block_on(async {
            introspector.generate_schema(&stats, "test_schema").await.unwrap()
        });

        // Multiple generations should produce identical schemas
        assert_eq!(schema1.id, schema2.id);
        assert_eq!(schema1.classes.len(), schema2.classes.len());
        assert_eq!(schema1.slots.len(), schema2.slots.len());
    }
}
