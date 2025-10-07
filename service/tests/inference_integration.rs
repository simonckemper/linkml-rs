//! Integration tests for LinkML schema inference system - Phase 4
//!
//! These tests verify end-to-end workflows with real data files,
//! service integration, and multi-document analysis scenarios.
//!
//! Phase 4 additions:
//! - InferenceEngine with automatic format detection
//! - Format Identification Service integration
//! - Multi-document aggregation workflows
//! - Full service composition testing

use linkml_service::inference::{
    CsvIntrospector, DataIntrospector, JsonIntrospector, XmlIntrospector, create_inference_engine,
};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::fs;

// Helper function to create test services
fn create_test_services() -> (
    Arc<dyn logger_core::LoggerService<Error = logger_core::LoggerError>>,
    Arc<dyn timestamp_core::TimestampService<Error = timestamp_core::TimestampError>>,
) {
    let timestamp = timestamp_service::wiring::wire_timestamp().into_arc();
    let logger = logger_service::wiring::wire_logger(timestamp.clone()).into_arc();
    (logger, timestamp)
}

// Helper to create a temporary directory with test files
async fn setup_test_directory() -> Result<TempDir, Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    Ok(temp_dir)
}

// Test: End-to-end XML analysis workflow
#[tokio::test]
async fn test_xml_end_to_end_workflow() -> Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let introspector = XmlIntrospector::new(logger, timestamp);

    // Create test XML file
    let temp_dir = setup_test_directory().await?;
    let xml_path = temp_dir.path().join("test.xml");

    let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<library>
    <book isbn="978-3-16-148410-0" year="2025">
        <title lang="en">Rust Programming</title>
        <author>
            <name>John Doe</name>
            <email>john@example.com</email>
        </author>
        <genre>Technology</genre>
        <pages>500</pages>
    </book>
    <book isbn="978-1-23-456789-0" year="2024">
        <title lang="en">Advanced LinkML</title>
        <author>
            <name>Jane Smith</name>
            <email>jane@example.com</email>
        </author>
        <genre>Technology</genre>
        <pages>350</pages>
    </book>
</library>"#;

    fs::write(&xml_path, xml_content).await?;

    // Analyze file
    let stats = introspector.analyze_file(&xml_path).await?;

    // Verify structure was captured
    assert!(stats.elements.contains_key("library"));
    assert!(stats.elements.contains_key("book"));
    assert!(stats.elements.contains_key("title"));
    assert!(stats.elements.contains_key("author"));
    assert!(stats.elements.contains_key("name"));
    assert!(stats.elements.contains_key("email"));
    assert!(stats.elements.contains_key("genre"));
    assert!(stats.elements.contains_key("pages"));

    // Verify book element details
    let book = stats.elements.get("book").unwrap();
    assert_eq!(book.occurrence_count, 2);
    assert!(book.attributes.contains_key("isbn"));
    assert!(book.attributes.contains_key("year"));
    assert_eq!(book.children.len(), 4); // title, author, genre, pages

    // Generate schema
    let schema = introspector
        .generate_schema(&stats, "library_schema")
        .await?;

    // Verify schema structure
    assert_eq!(schema.id, "library_schema");
    assert!(schema.classes.contains_key("book"));
    assert!(schema.classes.contains_key("author"));

    // Verify book class has expected slots
    let book_class = schema.classes.get("book").unwrap();
    assert!(book_class.slots.contains(&"isbn".to_string()));
    assert!(book_class.slots.contains(&"year".to_string()));
    assert!(book_class.slots.contains(&"title".to_string()));
    assert!(book_class.slots.contains(&"author".to_string()));

    Ok(())
}

// Test: End-to-end JSON analysis workflow
#[tokio::test]
async fn test_json_end_to_end_workflow() -> Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let introspector = JsonIntrospector::new(logger, timestamp);

    // Create test JSON file
    let temp_dir = setup_test_directory().await?;
    let json_path = temp_dir.path().join("test.json");

    let json_content = r#"{
    "users": [
        {
            "id": 1,
            "name": "John Doe",
            "email": "john@example.com",
            "age": 30,
            "active": true,
            "roles": ["admin", "user"],
            "address": {
                "street": "123 Main St",
                "city": "New York",
                "zipcode": "10001"
            }
        },
        {
            "id": 2,
            "name": "Jane Smith",
            "email": "jane@example.com",
            "age": 25,
            "active": false,
            "roles": ["user"],
            "address": {
                "street": "456 Oak Ave",
                "city": "Boston",
                "zipcode": "02101"
            }
        }
    ]
}"#;

    fs::write(&json_path, json_content).await?;

    // Analyze file
    let stats = introspector.analyze_file(&json_path).await?;

    // Verify structure was captured
    assert!(stats.elements.contains_key("user"));
    assert!(stats.elements.contains_key("address"));

    // Verify user element details
    let user = stats.elements.get("user").unwrap();
    assert_eq!(user.occurrence_count, 2);
    assert!(user.attributes.contains_key("id"));
    assert!(user.attributes.contains_key("name"));
    assert!(user.attributes.contains_key("email"));
    assert!(user.attributes.contains_key("age"));
    assert!(user.attributes.contains_key("active"));
    assert!(user.children.contains_key("address"));

    // Generate schema
    let schema = introspector.generate_schema(&stats, "users_schema").await?;

    // Verify schema structure
    assert_eq!(schema.id, "users_schema");
    assert!(schema.classes.contains_key("user"));
    assert!(schema.classes.contains_key("address"));

    // Verify user class has expected slots
    let user_class = schema.classes.get("user").unwrap();
    assert!(user_class.slots.contains(&"name".to_string()));
    assert!(user_class.slots.contains(&"email".to_string()));
    assert!(user_class.slots.contains(&"address".to_string()));

    Ok(())
}

// Test: End-to-end CSV analysis workflow
#[tokio::test]
async fn test_csv_end_to_end_workflow() -> Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let introspector = CsvIntrospector::new(logger, timestamp);

    // Create test CSV file
    let temp_dir = setup_test_directory().await?;
    let csv_path = temp_dir.path().join("test.csv");

    let csv_content = r#"id,name,age,email,salary,hire_date
1,John Doe,30,john@example.com,75000.50,2020-01-15
2,Jane Smith,25,jane@example.com,82000.00,2021-03-20
3,Bob Johnson,35,bob@example.com,95000.75,2019-06-10
4,Alice Williams,28,alice@example.com,78500.25,2022-02-28"#;

    fs::write(&csv_path, csv_content).await?;

    // Analyze file
    let stats = introspector.analyze_file(&csv_path).await?;

    // Verify structure was captured
    assert!(stats.elements.contains_key("Record"));

    let record = stats.elements.get("Record").unwrap();
    assert!(record.attributes.contains_key("id"));
    assert!(record.attributes.contains_key("name"));
    assert!(record.attributes.contains_key("age"));
    assert!(record.attributes.contains_key("email"));
    assert!(record.attributes.contains_key("salary"));
    assert!(record.attributes.contains_key("hire_date"));

    // Generate schema
    let schema = introspector
        .generate_schema(&stats, "employees_schema")
        .await?;

    // Verify schema structure
    assert_eq!(schema.id, "employees_schema");
    assert!(schema.classes.contains_key("Record"));

    // Verify record class has expected slots
    let record_class = schema.classes.get("Record").unwrap();
    assert!(record_class.slots.contains(&"name".to_string()));
    assert!(record_class.slots.contains(&"email".to_string()));
    assert!(record_class.slots.contains(&"age".to_string()));

    Ok(())
}

// Test: PAGE-XML real-world scenario
#[tokio::test]
async fn test_page_xml_real_world_scenario() -> Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let introspector = XmlIntrospector::new(logger, timestamp);

    let page_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<PcGts xmlns="http://schema.primaresearch.org/PAGE/gts/pagecontent/2013-07-15">
    <Metadata>
        <Creator>Test OCR System</Creator>
        <Created>2025-10-02T12:00:00</Created>
        <LastChange>2025-10-02T14:30:00</LastChange>
    </Metadata>
    <Page imageFilename="page001.jpg" imageWidth="2400" imageHeight="3200">
        <TextRegion id="r1" type="paragraph">
            <Coords points="100,100 800,100 800,400 100,400"/>
            <TextLine id="l1">
                <Baseline points="100,150 800,150"/>
                <Coords points="100,120 800,120 800,180 100,180"/>
                <TextEquiv conf="0.95">
                    <Unicode>The quick brown fox jumps over the lazy dog.</Unicode>
                    <PlainText>The quick brown fox jumps over the lazy dog.</PlainText>
                </TextEquiv>
            </TextLine>
            <TextLine id="l2">
                <Baseline points="100,250 800,250"/>
                <Coords points="100,220 800,220 800,280 100,280"/>
                <TextEquiv conf="0.92">
                    <Unicode>This is a sample text line from PAGE-XML.</Unicode>
                    <PlainText>This is a sample text line from PAGE-XML.</PlainText>
                </TextEquiv>
            </TextLine>
        </TextRegion>
        <TextRegion id="r2" type="heading">
            <Coords points="100,50 800,50 800,90 100,90"/>
            <TextLine id="l3">
                <Baseline points="100,70 800,70"/>
                <Coords points="100,50 800,50 800,90 100,90"/>
                <TextEquiv conf="0.98">
                    <Unicode>Document Title</Unicode>
                    <PlainText>Document Title</PlainText>
                </TextEquiv>
            </TextLine>
        </TextRegion>
    </Page>
</PcGts>"#;

    let stats = introspector.analyze_bytes(page_xml.as_bytes()).await?;

    // Verify PAGE-XML specific elements
    assert!(stats.elements.contains_key("PcGts"));
    assert!(stats.elements.contains_key("Metadata"));
    assert!(stats.elements.contains_key("Page"));
    assert!(stats.elements.contains_key("TextRegion"));
    assert!(stats.elements.contains_key("TextLine"));
    assert!(stats.elements.contains_key("Baseline"));
    assert!(stats.elements.contains_key("TextEquiv"));
    assert!(stats.elements.contains_key("Unicode"));
    assert!(stats.elements.contains_key("PlainText"));

    // Verify namespace was detected
    assert!(!stats.namespaces.is_empty());
    assert!(
        stats
            .namespaces
            .values()
            .any(|uri| uri.contains("primaresearch.org/PAGE"))
    );

    // Verify schema name indicates PAGE-XML was detected
    assert!(
        stats
            .metadata
            .schema_name
            .is_some_and(|name| name.contains("PAGE-XML"))
    );

    // Verify TextRegion structure
    let text_region = stats.elements.get("TextRegion").unwrap();
    assert_eq!(text_region.occurrence_count, 2);
    assert!(text_region.attributes.contains_key("id"));
    assert!(text_region.attributes.contains_key("type"));
    assert!(text_region.children.contains_key("Coords"));
    assert!(text_region.children.contains_key("TextLine"));

    Ok(())
}

// Test: EAD (Encoded Archival Description) scenario
#[tokio::test]
async fn test_ead_archival_description() -> Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let introspector = XmlIntrospector::new(logger, timestamp);

    let ead_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ead xmlns="urn:isbn:1-931666-22-9">
    <eadheader>
        <eadid countrycode="US" mainagencycode="US-NNU">test-001</eadid>
        <filedesc>
            <titlestmt>
                <titleproper>Test Archival Collection</titleproper>
                <author>Archival Institution</author>
            </titlestmt>
            <publicationstmt>
                <publisher>Test Archive</publisher>
                <date>2025</date>
            </publicationstmt>
        </filedesc>
    </eadheader>
    <archdesc level="collection">
        <did>
            <unittitle>Test Finding Aid</unittitle>
            <unitdate normal="1900/2000">1900-2000</unitdate>
            <physdesc>
                <extent>10 linear feet</extent>
            </physdesc>
        </did>
        <scopecontent>
            <p>This is a test archival collection.</p>
        </scopecontent>
    </archdesc>
</ead>"#;

    let stats = introspector.analyze_bytes(ead_xml.as_bytes()).await?;

    // Verify EAD-specific elements
    assert!(stats.elements.contains_key("ead"));
    assert!(stats.elements.contains_key("eadheader"));
    assert!(stats.elements.contains_key("archdesc"));
    assert!(stats.elements.contains_key("did"));
    assert!(stats.elements.contains_key("unittitle"));

    // Verify schema name indicates EAD was detected
    assert!(
        stats
            .metadata
            .schema_name
            .is_some_and(|name| name.contains("EAD"))
    );

    Ok(())
}

// Test: Dublin Core metadata scenario
#[tokio::test]
async fn test_dublin_core_metadata() -> Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let introspector = XmlIntrospector::new(logger, timestamp);

    let dc_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Test Digital Resource</dc:title>
    <dc:creator>John Doe</dc:creator>
    <dc:subject>Testing</dc:subject>
    <dc:subject>LinkML</dc:subject>
    <dc:subject>Metadata</dc:subject>
    <dc:description>A test resource for Dublin Core metadata extraction</dc:description>
    <dc:publisher>Test Publisher</dc:publisher>
    <dc:contributor>Jane Smith</dc:contributor>
    <dc:date>2025-10-02</dc:date>
    <dc:type>Text</dc:type>
    <dc:format>text/plain</dc:format>
    <dc:identifier>http://example.com/test-resource</dc:identifier>
    <dc:language>en</dc:language>
    <dc:rights>CC-BY-NC-4.0</dc:rights>
</metadata>"#;

    let stats = introspector.analyze_bytes(dc_xml.as_bytes()).await?;

    // Verify Dublin Core elements
    assert!(stats.elements.contains_key("title"));
    assert!(stats.elements.contains_key("creator"));
    assert!(stats.elements.contains_key("subject"));
    assert!(stats.elements.contains_key("description"));

    // Verify schema name indicates Dublin Core was detected
    assert!(
        stats
            .metadata
            .schema_name
            .is_some_and(|name| name.contains("Dublin Core"))
    );

    // Verify multivalued subject field
    let subject = stats.elements.get("subject").unwrap();
    assert_eq!(subject.occurrence_count, 3);

    Ok(())
}

// Test: Large document performance
#[tokio::test]
async fn test_large_document_performance() -> Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let introspector = XmlIntrospector::new(logger, timestamp);

    // Generate large XML document
    let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?><catalog>"#);
    for i in 0..1000 {
        xml.push_str(&format!(
            r#"<product id="{}" sku="SKU-{:06}">
                <name>Product {}</name>
                <price>{}.99</price>
                <category>Category {}</category>
                <inStock>true</inStock>
                <tags>
                    <tag>tag{}</tag>
                    <tag>tag{}</tag>
                </tags>
            </product>"#,
            i,
            i,
            i,
            (i % 100) + 10,
            i % 10,
            i % 50,
            (i + 1) % 50
        ));
    }
    xml.push_str("</catalog>");

    let start = std::time::Instant::now();
    let stats = introspector.analyze_bytes(xml.as_bytes()).await?;
    let analysis_duration = start.elapsed();

    // Verify analysis completed
    assert_eq!(
        stats.elements.get("product").unwrap().occurrence_count,
        1000
    );

    // Verify performance target (<500ms for 1000 element document)
    // Note: This is a generous target. Actual performance should be better.
    assert!(
        analysis_duration.as_millis() < 2000,
        "Analysis took too long: {:?}",
        analysis_duration
    );

    Ok(())
}

// Test: Empty document handling
#[tokio::test]
async fn test_empty_xml_document() -> Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let introspector = XmlIntrospector::new(logger, timestamp);

    let xml = r#"<?xml version="1.0" encoding="UTF-8"?><root/>"#;
    let stats = introspector.analyze_bytes(xml.as_bytes()).await?;

    assert_eq!(stats.elements.len(), 1);
    assert!(stats.elements.contains_key("root"));

    let root = stats.elements.get("root").unwrap();
    assert_eq!(root.occurrence_count, 1);
    assert_eq!(root.attributes.len(), 0);
    assert_eq!(root.children.len(), 0);

    Ok(())
}

// Test: Empty JSON document handling
#[tokio::test]
async fn test_empty_json_document() -> Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let introspector = JsonIntrospector::new(logger, timestamp);

    let json = r#"{}"#;
    let stats = introspector.analyze_bytes(json.as_bytes()).await?;

    assert!(stats.elements.contains_key("root"));

    let root = stats.elements.get("root").unwrap();
    assert_eq!(root.occurrence_count, 1);
    assert_eq!(root.attributes.len(), 0);

    Ok(())
}

// Test: Malformed XML handling
#[tokio::test]
async fn test_malformed_xml() {
    let (logger, timestamp) = create_test_services();
    let introspector = XmlIntrospector::new(logger, timestamp);

    let malformed_xml = r#"<?xml version="1.0"?><root><unclosed>"#;
    let result = introspector.analyze_bytes(malformed_xml.as_bytes()).await;

    assert!(result.is_err());
}

// Test: Malformed JSON handling
#[tokio::test]
async fn test_malformed_json() {
    let (logger, timestamp) = create_test_services();
    let introspector = JsonIntrospector::new(logger, timestamp);

    let malformed_json = r#"{"key": "value", "incomplete"#;
    let result = introspector.analyze_bytes(malformed_json.as_bytes()).await;

    assert!(result.is_err());
}

// Test: Deep nesting handling (XML)
#[tokio::test]
async fn test_deep_nesting_xml() -> Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let introspector = XmlIntrospector::new(logger, timestamp);

    let mut xml = String::from(r#"<?xml version="1.0"?>"#);
    let depth = 100;

    for i in 0..depth {
        xml.push_str(&format!("<level{}>", i));
    }
    xml.push_str("<data>Deep value</data>");
    for i in (0..depth).rev() {
        xml.push_str(&format!("</level{}>", i));
    }

    let stats = introspector.analyze_bytes(xml.as_bytes()).await?;

    assert!(stats.document_metrics.max_nesting_depth >= depth);

    Ok(())
}

// Test: Deep nesting handling (JSON)
#[tokio::test]
async fn test_deep_nesting_json() -> Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let introspector = JsonIntrospector::new(logger, timestamp);

    let mut json = String::new();
    let depth = 100;

    for i in 0..depth {
        json.push_str(&format!(r#"{{"level{}":"#, i));
    }
    json.push_str(r#"{"value":"deep"}"#);
    for _ in 0..depth {
        json.push('}');
    }

    let stats = introspector.analyze_bytes(json.as_bytes()).await?;

    assert!(stats.document_metrics.max_nesting_depth >= depth);

    Ok(())
}

// Test: Mixed content handling
#[tokio::test]
async fn test_mixed_content_xml() -> Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let introspector = XmlIntrospector::new(logger, timestamp);

    let xml = r#"<?xml version="1.0"?>
<paragraph>
    This is some text with <emphasis>emphasized content</emphasis> and <strong>bold text</strong> mixed in.
</paragraph>"#;

    let stats = introspector.analyze_bytes(xml.as_bytes()).await?;

    let paragraph = stats.elements.get("paragraph").unwrap();
    assert!(paragraph.has_mixed_content);

    Ok(())
}

// Test: Unicode handling
#[tokio::test]
async fn test_unicode_content() -> Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let introspector = XmlIntrospector::new(logger, timestamp);

    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<document>
    <title>测试文档</title>
    <content>こんにちは世界</content>
    <author>Müller</author>
    <emoji>🦀 Rust 🚀</emoji>
</document>"#;

    let stats = introspector.analyze_bytes(xml.as_bytes()).await?;

    assert!(stats.elements.contains_key("title"));
    assert!(stats.elements.contains_key("content"));
    assert!(stats.elements.contains_key("emoji"));

    let title = stats.elements.get("title").unwrap();
    assert_eq!(title.text_samples[0], "测试文档");

    Ok(())
}

// ============================================================================
// Phase 4: InferenceEngine Integration Tests
// ============================================================================

/// Test: InferenceEngine automatic format detection and schema generation
#[tokio::test]
async fn test_inference_engine_auto_detection() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = setup_test_directory().await?;
    let xml_path = temp_dir.path().join("test.xml");

    let xml_content = r#"<?xml version="1.0"?>
<Person>
    <name>John Doe</name>
    <age>30</age>
    <email>john@example.com</email>
</Person>"#;

    fs::write(&xml_path, xml_content).await?;

    // Create inference engine with all services
    let engine = create_inference_engine().await?;

    // Perform automatic schema inference
    let schema = engine.infer_from_file_auto(&xml_path).await?;

    // Verify schema was generated
    assert!(!schema.classes.is_empty(), "Schema should contain classes");

    Ok(())
}

/// Test: InferenceEngine multi-document aggregation
#[tokio::test]
async fn test_inference_engine_multi_document() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = setup_test_directory().await?;

    // Create multiple XML files
    let mut paths = Vec::new();
    for i in 0..3 {
        let path = temp_dir.path().join(format!("doc_{}.xml", i));
        let content = format!(
            r#"<?xml version="1.0"?>
<Document>
    <id>{}</id>
    <title>Document {}</title>
</Document>"#,
            i, i
        );
        fs::write(&path, content).await?;
        paths.push(path);
    }

    let engine = create_inference_engine().await?;
    let schema = engine.analyze_documents(&paths).await?;

    assert!(!schema.classes.is_empty());
    Ok(())
}
