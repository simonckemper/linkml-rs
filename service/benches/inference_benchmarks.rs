// Copyright (C) 2025 Simon C. Kemper
// Licensed under Creative Commons BY-NC 4.0
//
// Comprehensive benchmarks for LinkML schema inference system

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use linkml_service::inference::{
    CsvIntrospector, DataIntrospector, JsonIntrospector, XmlIntrospector,
};
use timestamp_service::create_timestamp_service;

// Test data generators for realistic benchmarking

fn generate_simple_xml(elements: usize) -> String {
    let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?><root>"#);
    for i in 0..elements {
        xml.push_str(&format!(
            r#"<person id="{}"><name>Person {}</name><age>{}</age><email>person{}@example.com</email></person>"#,
            i, i, 25 + (i % 50), i
        ));
    }
    xml.push_str("</root>");
    xml
}

fn generate_nested_xml(depth: usize) -> String {
    let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    for i in 0..depth {
        xml.push_str(&format!(r#"<level{}>"#, i));
    }
    xml.push_str("<data>Deep content</data>");
    for i in (0..depth).rev() {
        xml.push_str(&format!(r#"</level{}>"#, i));
    }
    xml
}

fn generate_page_xml() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8"?>
    <PcGts xmlns="http://schema.primaresearch.org/PAGE/gts/pagecontent/2013-07-15">
        <Metadata>
            <Creator>Test Creator</Creator>
            <Created>2025-10-02T12:00:00</Created>
        </Metadata>
        <Page imageFilename="test.jpg" imageWidth="2000" imageHeight="3000">
            <TextRegion id="r1" type="paragraph">
                <Coords points="100,100 500,100 500,300 100,300"/>
                <TextLine id="l1">
                    <Baseline points="100,150 500,150"/>
                    <Coords points="100,120 500,120 500,180 100,180"/>
                    <TextEquiv>
                        <Unicode>Sample text line 1</Unicode>
                        <PlainText>Sample text line 1</PlainText>
                    </TextEquiv>
                </TextLine>
                <TextLine id="l2">
                    <Baseline points="100,250 500,250"/>
                    <Coords points="100,220 500,220 500,280 100,280"/>
                    <TextEquiv>
                        <Unicode>Sample text line 2</Unicode>
                        <PlainText>Sample text line 2</PlainText>
                    </TextEquiv>
                </TextLine>
            </TextRegion>
            <TextRegion id="r2" type="heading">
                <Coords points="100,50 500,50 500,90 100,90"/>
                <TextLine id="l3">
                    <Baseline points="100,70 500,70"/>
                    <Coords points="100,50 500,50 500,90 100,90"/>
                    <TextEquiv>
                        <Unicode>Document Title</Unicode>
                        <PlainText>Document Title</PlainText>
                    </TextEquiv>
                </TextLine>
            </TextRegion>
        </Page>
    </PcGts>"#
}

fn generate_simple_json(objects: usize) -> String {
    let mut json = String::from(r#"{"people":["#);
    for i in 0..objects {
        if i > 0 {
            json.push(',');
        }
        json.push_str(&format!(
            r#"{{"name":"Person {}","age":{},"email":"person{}@example.com","active":{}}}"#,
            i,
            25 + (i % 50),
            i,
            i % 2 == 0
        ));
    }
    json.push_str("]}");
    json
}

fn generate_nested_json(depth: usize) -> String {
    let mut json = String::new();
    for i in 0..depth {
        json.push_str(&format!(r#"{{"level{}":"#, i));
    }
    json.push_str(r#"{"value":"deep"}"#);
    for _ in 0..depth {
        json.push('}');
    }
    json
}

fn generate_complex_json() -> &'static str {
    r#"{
        "schema": {
            "id": "test_schema",
            "name": "Test Schema",
            "version": "1.0.0",
            "classes": [
                {
                    "name": "Person",
                    "description": "A person entity",
                    "slots": ["name", "age", "email"],
                    "attributes": {
                        "name": {"type": "string", "required": true},
                        "age": {"type": "integer", "required": false},
                        "email": {"type": "string", "pattern": "^[^@]+@[^@]+\\.[^@]+$"}
                    }
                },
                {
                    "name": "Address",
                    "description": "An address entity",
                    "slots": ["street", "city", "zipcode"],
                    "attributes": {
                        "street": {"type": "string", "required": true},
                        "city": {"type": "string", "required": true},
                        "zipcode": {"type": "string", "pattern": "^\\d{5}$"}
                    }
                }
            ],
            "enums": [
                {
                    "name": "Status",
                    "values": ["active", "inactive", "pending"]
                }
            ]
        }
    }"#
}

fn generate_simple_csv(rows: usize) -> String {
    let mut csv = String::from("id,name,age,email,active\n");
    for i in 0..rows {
        csv.push_str(&format!(
            "{},Person {},\"{}\",person{}@example.com,{}\n",
            i,
            i,
            25 + (i % 50),
            i,
            i % 2 == 0
        ));
    }
    csv
}

// Benchmark: XML introspector with varying document sizes
fn bench_xml_introspector_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("xml_introspector_sizes");

    for size in [10, 50, 100, 500].iter() {
        let xml = generate_simple_xml(*size);
        let data = xml.as_bytes();

        group.throughput(Throughput::Bytes(data.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let logger = create_logger_service().unwrap();
            let timestamp = wire_timestamp();
            let introspector = XmlIntrospector::new(logger, timestamp);

            b.to_async(&rt)
                .iter(|| async { introspector.analyze_bytes(black_box(data)).await.unwrap() });
        });
    }

    group.finish();
}

// Benchmark: XML introspector with varying nesting depths
fn bench_xml_introspector_depth(c: &mut Criterion) {
    let mut group = c.benchmark_group("xml_introspector_depth");

    for depth in [5, 10, 20, 50].iter() {
        let xml = generate_nested_xml(*depth);
        let data = xml.as_bytes();

        group.bench_with_input(BenchmarkId::from_parameter(depth), depth, |b, _| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let logger = create_logger_service().unwrap();
            let timestamp = wire_timestamp();
            let introspector = XmlIntrospector::new(logger, timestamp);

            b.to_async(&rt)
                .iter(|| async { introspector.analyze_bytes(black_box(data)).await.unwrap() });
        });
    }

    group.finish();
}

// Benchmark: PAGE-XML analysis (realistic GLAM use case)
fn bench_xml_page_xml_analysis(c: &mut Criterion) {
    c.bench_function("xml_page_xml_real_world", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let logger = create_logger_service().unwrap();
        let timestamp = wire_timestamp();
        let introspector = XmlIntrospector::new(logger, timestamp);
        let data = generate_page_xml().as_bytes();

        b.to_async(&rt)
            .iter(|| async { introspector.analyze_bytes(black_box(data)).await.unwrap() });
    });
}

// Benchmark: XML schema generation
fn bench_xml_schema_generation(c: &mut Criterion) {
    c.bench_function("xml_schema_generation", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let logger = create_logger_service().unwrap();
        let timestamp = wire_timestamp();
        let introspector = XmlIntrospector::new(logger, timestamp);
        let xml = generate_simple_xml(100);

        let stats =
            rt.block_on(async { introspector.analyze_bytes(xml.as_bytes()).await.unwrap() });

        b.to_async(&rt).iter(|| async {
            introspector
                .generate_schema(black_box(&stats), "bench_schema")
                .await
                .unwrap()
        });
    });
}

// Benchmark: JSON introspector with varying document sizes
fn bench_json_introspector_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_introspector_sizes");

    for size in [10, 50, 100, 500].iter() {
        let json = generate_simple_json(*size);
        let data = json.as_bytes();

        group.throughput(Throughput::Bytes(data.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let logger = create_logger_service().unwrap();
            let timestamp = wire_timestamp();
            let introspector = JsonIntrospector::new(logger, timestamp);

            b.to_async(&rt)
                .iter(|| async { introspector.analyze_bytes(black_box(data)).await.unwrap() });
        });
    }

    group.finish();
}

// Benchmark: JSON introspector with varying nesting depths
fn bench_json_introspector_depth(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_introspector_depth");

    for depth in [5, 10, 20, 50].iter() {
        let json = generate_nested_json(*depth);
        let data = json.as_bytes();

        group.bench_with_input(BenchmarkId::from_parameter(depth), depth, |b, _| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let logger = create_logger_service().unwrap();
            let timestamp = wire_timestamp();
            let introspector = JsonIntrospector::new(logger, timestamp);

            b.to_async(&rt)
                .iter(|| async { introspector.analyze_bytes(black_box(data)).await.unwrap() });
        });
    }

    group.finish();
}

// Benchmark: Complex JSON schema analysis
fn bench_json_complex_schema(c: &mut Criterion) {
    c.bench_function("json_complex_schema_analysis", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let logger = create_logger_service().unwrap();
        let timestamp = wire_timestamp();
        let introspector = JsonIntrospector::new(logger, timestamp);
        let data = generate_complex_json().as_bytes();

        b.to_async(&rt)
            .iter(|| async { introspector.analyze_bytes(black_box(data)).await.unwrap() });
    });
}

// Benchmark: JSON schema generation
fn bench_json_schema_generation(c: &mut Criterion) {
    c.bench_function("json_schema_generation", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let logger = create_logger_service().unwrap();
        let timestamp = wire_timestamp();
        let introspector = JsonIntrospector::new(logger, timestamp);
        let json = generate_simple_json(100);

        let stats =
            rt.block_on(async { introspector.analyze_bytes(json.as_bytes()).await.unwrap() });

        b.to_async(&rt).iter(|| async {
            introspector
                .generate_schema(black_box(&stats), "bench_schema")
                .await
                .unwrap()
        });
    });
}

// Benchmark: CSV introspector with varying row counts
fn bench_csv_introspector_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("csv_introspector_sizes");

    for size in [10, 50, 100, 500, 1000].iter() {
        let csv = generate_simple_csv(*size);
        let data = csv.as_bytes();

        group.throughput(Throughput::Bytes(data.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let logger = create_logger_service().unwrap();
            let timestamp = wire_timestamp();
            let introspector = CsvIntrospector::new(logger, timestamp);

            b.to_async(&rt)
                .iter(|| async { introspector.analyze_bytes(black_box(data)).await.unwrap() });
        });
    }

    group.finish();
}

// Benchmark: End-to-end XML analysis (analyze + generate schema)
fn bench_xml_end_to_end(c: &mut Criterion) {
    c.bench_function("xml_end_to_end_100_elements", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let logger = create_logger_service().unwrap();
        let timestamp = wire_timestamp();
        let introspector = XmlIntrospector::new(logger, timestamp);
        let xml = generate_simple_xml(100);
        let data = xml.as_bytes();

        b.to_async(&rt).iter(|| async {
            let stats = introspector.analyze_bytes(black_box(data)).await.unwrap();
            introspector
                .generate_schema(&stats, "bench_schema")
                .await
                .unwrap()
        });
    });
}

// Benchmark: End-to-end JSON analysis (analyze + generate schema)
fn bench_json_end_to_end(c: &mut Criterion) {
    c.bench_function("json_end_to_end_100_objects", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let logger = create_logger_service().unwrap();
        let timestamp = wire_timestamp();
        let introspector = JsonIntrospector::new(logger, timestamp);
        let json = generate_simple_json(100);
        let data = json.as_bytes();

        b.to_async(&rt).iter(|| async {
            let stats = introspector.analyze_bytes(black_box(data)).await.unwrap();
            introspector
                .generate_schema(&stats, "bench_schema")
                .await
                .unwrap()
        });
    });
}

criterion_group!(
    benches,
    bench_xml_introspector_sizes,
    bench_xml_introspector_depth,
    bench_xml_page_xml_analysis,
    bench_xml_schema_generation,
    bench_xml_end_to_end,
    bench_json_introspector_sizes,
    bench_json_introspector_depth,
    bench_json_complex_schema,
    bench_json_schema_generation,
    bench_json_end_to_end,
    bench_csv_introspector_sizes,
);

criterion_main!(benches);
