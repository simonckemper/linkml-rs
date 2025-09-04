//! Comprehensive performance benchmarks for LinkML service

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use linkml_core::types::SchemaDefinition;
use linkml_service::factory::create_linkml_service;
use linkml_service::generator::python_dataclass::PythonDataclassGenerator;
use linkml_service::generator::traits::Generator;
use linkml_service::generator::typescript::TypeScriptGenerator;
use linkml_service::parser::yaml_parser::YamlParser;
use linkml_service::validator::ValidationEngine;
use serde_json::json;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Create a complex schema for benchmarking
fn create_complex_schema() -> SchemaDefinition {
    let schema_yaml = r#"
id: https://example.org/benchmark-schema
name: benchmark_schema
description: Complex schema for performance benchmarking

classes:
  Person:
    description: A person entity
    slots:
      - id
      - name
      - email
      - age
      - addresses
      - publications
      - affiliations
    slot_usage:
      id:
        identifier: true
        required: true
      addresses:
        multivalued: true
        range: Address
      publications:
        multivalued: true
        range: Publication
      affiliations:
        multivalued: true
        range: Organization

  Address:
    description: A postal address
    slots:
      - id
      - street
      - city
      - state
      - postal_code
      - country
    slot_usage:
      id:
        identifier: true
        required: true

  Publication:
    description: A scientific publication
    slots:
      - id
      - title
      - abstract
      - authors
      - journal
      - publication_date
      - doi
      - keywords
    slot_usage:
      id:
        identifier: true
        required: true
      authors:
        multivalued: true
        range: Person
      keywords:
        multivalued: true

  Organization:
    description: An organization
    slots:
      - id
      - name
      - type
      - address
      - website
      - employees
    slot_usage:
      id:
        identifier: true
        required: true
      employees:
        multivalued: true
        range: Person

  Journal:
    description: A scientific journal
    slots:
      - id
      - name
      - issn
      - impact_factor
      - publisher
    slot_usage:
      id:
        identifier: true
        required: true

slots:
  id:
    range: string
    required: true
  name:
    range: string
    required: true
  email:
    range: string
    pattern: "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"
  age:
    range: integer
    minimum_value: 0
    maximum_value: 150
  street:
    range: string
  city:
    range: string
  state:
    range: string
  postal_code:
    range: string
    pattern: "^\\d{5}(-\\d{4})?$"
  country:
    range: string
  title:
    range: string
  abstract:
    range: string
  authors:
    range: Person
    multivalued: true
  journal:
    range: Journal
  publication_date:
    range: date
  doi:
    range: string
    pattern: "^10\\.\\d+/.+"
  keywords:
    range: string
    multivalued: true
  type:
    range: OrganizationType
  address:
    range: Address
  website:
    range: string
    pattern: "^https?://.*"
  employees:
    range: Person
    multivalued: true
  addresses:
    range: Address
    multivalued: true
  publications:
    range: Publication
    multivalued: true
  affiliations:
    range: Organization
    multivalued: true
  issn:
    range: string
    pattern: "^\\d{4}-\\d{3}[\\dX]$"
  impact_factor:
    range: float
    minimum_value: 0.0
  publisher:
    range: Organization

enums:
  OrganizationType:
    permissible_values:
      university:
        description: Academic institution
      company:
        description: Commercial organization
      government:
        description: Government agency
      nonprofit:
        description: Non-profit organization

types:
  string:
    base: str
  integer:
    base: int
  float:
    base: float
  date:
    base: str
    description: Date in ISO 8601 format
"#;

    let parser = YamlParser::new();
    parser
        .parse_str(schema_yaml)
        .expect("Failed to parse benchmark schema")
}

/// Generate test data for benchmarking
fn generate_test_data(count: usize) -> Vec<serde_json::Value> {
    let mut data = Vec::with_capacity(count);

    for i in 0..count {
        let person = json!({
            "id": format!("person:{:06}", i),
            "name": format!("Person {}", i),
            "email": format!("person{}@example.com", i),
            "age": 25 + (i % 50),
            "addresses": [{
                "id": format!("addr:{:06}", i),
                "street": format!("{} Main St", 100 + i),
                "city": "Example City",
                "state": "CA",
                "postal_code": format!("{:05}", 90000 + (i % 1000)),
                "country": "USA"
            }],
            "publications": [{
                "id": format!("pub:{:06}", i),
                "title": format!("Research Paper {}", i),
                "abstract": "This is a sample abstract for benchmarking purposes.",
                "authors": [format!("person:{:06}", i)],
                "publication_date": "2023-01-01",
                "doi": format!("10.1234/example.{}", i),
                "keywords": ["research", "science", "benchmark"]
            }],
            "affiliations": [{
                "id": format!("org:{:06}", i % 10),
                "name": format!("Organization {}", i % 10),
                "type": match i % 4 {
                    0 => "university",
                    1 => "company",
                    2 => "government",
                    _ => "nonprofit"
                },
                "website": format!("https://org{}.example.com", i % 10)
            }]
        });
        data.push(person);
    }

    data
}

/// Benchmark schema parsing performance
fn bench_schema_parsing(c: &mut Criterion) {
    let schema_yaml = r#"
id: https://example.org/simple-schema
name: simple_schema
classes:
  Person:
    slots: [id, name, email]
slots:
  id: {range: string, required: true}
  name: {range: string}
  email: {range: string}
types:
  string: {base: str}
"#;

    c.bench_function("schema_parsing_simple", |b| {
        let parser = YamlParser::new();
        b.iter(|| black_box(parser.parse_str(schema_yaml).unwrap()))
    });

    let complex_schema = create_complex_schema();
    let complex_yaml = serde_yaml::to_string(&complex_schema).unwrap();

    c.bench_function("schema_parsing_complex", |b| {
        let parser = YamlParser::new();
        b.iter(|| black_box(parser.parse_str(&complex_yaml).unwrap()))
    });
}

/// Benchmark validation performance
fn bench_validation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let schema = Arc::new(create_complex_schema());
    let engine = ValidationEngine::new(schema);

    let mut group = c.benchmark_group("validation");

    for size in [1, 10, 100, 1000].iter() {
        let test_data = generate_test_data(*size);

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::new("person_validation", size), size, |b, _| {
            b.iter(|| {
                rt.block_on(async {
                    for person in &test_data {
                        let result = engine
                            .validate_instance(black_box(person), "Person")
                            .await
                            .unwrap();
                        black_box(result);
                    }
                })
            })
        });
    }

    group.finish();
}

/// Benchmark code generation performance
fn bench_code_generation(c: &mut Criterion) {
    let schema = create_complex_schema();

    let mut group = c.benchmark_group("code_generation");

    group.bench_function("python_dataclass", |b| {
        let generator = PythonDataclassGenerator::new();
        b.iter(|| black_box(generator.generate(&schema).unwrap()))
    });

    group.bench_function("typescript", |b| {
        let generator = TypeScriptGenerator::new();
        b.iter(|| black_box(generator.generate(&schema).unwrap()))
    });

    group.finish();
}

/// Benchmark memory usage patterns
fn bench_memory_usage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let schema = Arc::new(create_complex_schema());

    let mut group = c.benchmark_group("memory_usage");

    // Test memory efficiency with different data sizes
    for size in [100, 1000, 10000].iter() {
        let test_data = generate_test_data(*size);

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::new("validation_memory", size), size, |b, _| {
            b.iter(|| {
                rt.block_on(async {
                    let engine = ValidationEngine::new(Arc::clone(&schema));
                    for person in &test_data {
                        let result = engine
                            .validate_instance(black_box(person), "Person")
                            .await
                            .unwrap();
                        black_box(result);
                    }
                })
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_schema_parsing,
    bench_validation,
    bench_code_generation,
    bench_memory_usage
);
criterion_main!(benches);
