//! Comprehensive performance benchmarks for the `LinkML` service.

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use linkml_core::types::SchemaDefinition;
use linkml_service::generator::python_dataclass::PythonDataclassGenerator;
use linkml_service::generator::traits::Generator;
use linkml_service::generator::typescript::TypeScriptGenerator;
use linkml_service::parser::{SchemaParser, yaml_parser::YamlParser};
use linkml_service::validator::ValidationEngine;
use serde_json::json;
use std::fmt::Display;
use std::sync::Arc;
use tokio::runtime::Runtime;

fn require_ok<T, E>(result: Result<T, E>, context: &str) -> T
where
    E: Display,
{
    match result {
        Ok(value) => value,
        Err(err) => panic!("{context}: {err}"),
    }
}

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
    require_ok(
        parser.parse_str(schema_yaml),
        "Failed to parse benchmark schema",
    )
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
/// Benchmark parsing representative schemas of different sizes.
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
        b.iter(|| {
            let parser = YamlParser::new();
            let schema = require_ok(
                parser.parse_str(schema_yaml),
                "Schema parsing benchmark should succeed",
            );
            black_box(schema)
        })
    });

    let complex_schema = create_complex_schema();
    let complex_yaml = require_ok(
        serde_yaml::to_string(&complex_schema),
        "Complex schema serialization should succeed",
    );

    c.bench_function("schema_parsing_complex", |b| {
        b.iter(|| {
            let parser = YamlParser::new();
            let schema = require_ok(
                parser.parse_str(&complex_yaml),
                "Complex schema parsing should succeed",
            );
            black_box(schema)
        })
    });
}

/// Benchmark validation performance
/// Benchmark validation performance for varied data shapes.
fn bench_validation(c: &mut Criterion) {
    let rt = require_ok(Runtime::new(), "Tokio runtime creation failed");
    let schema = Arc::new(create_complex_schema());
    let engine = Arc::new(require_ok(
        ValidationEngine::new(schema.as_ref()),
        "Validation engine creation failed",
    ));

    let mut group = c.benchmark_group("validation");

    for size in [1, 10, 100, 1000].iter() {
        let test_data = generate_test_data(*size);

        group.throughput(Throughput::Elements(u64::try_from(*size).unwrap_or(0)));
        group.bench_with_input(BenchmarkId::new("person_validation", size), size, |b, _| {
            b.iter(|| {
                let engine_ref = Arc::clone(&engine);
                let data = test_data.clone();
                rt.block_on(async move {
                    for person in &data {
                        let report = require_ok(
                            engine_ref
                                .validate_as_class(black_box(person), "Person", None)
                                .await,
                            "Validation should succeed",
                        );
                        black_box(report);
                    }
                })
            })
        });
    }

    group.finish();
}

/// Benchmark code generation performance
/// Benchmark code generation across multiple targets.
fn bench_code_generation(c: &mut Criterion) {
    let schema = create_complex_schema();

    let mut group = c.benchmark_group("code_generation");

    group.bench_function("python_dataclass", |b| {
        let generator = PythonDataclassGenerator::new();
        b.iter(|| {
            let output = require_ok(
                generator.generate(&schema),
                "Python dataclass generation should succeed",
            );
            black_box(output)
        })
    });

    group.bench_function("typescript", |b| {
        let generator = TypeScriptGenerator::new();
        b.iter(|| {
            let output = require_ok(
                generator.generate(&schema),
                "TypeScript generation should succeed",
            );
            black_box(output)
        })
    });

    group.finish();
}

/// Benchmark memory usage patterns
/// Benchmark memory usage during schema processing.
fn bench_memory_usage(c: &mut Criterion) {
    let rt = require_ok(Runtime::new(), "Tokio runtime creation failed");
    let schema = Arc::new(create_complex_schema());

    let mut group = c.benchmark_group("memory_usage");

    // Test memory efficiency with different data sizes
    for size in [100, 1000, 10000].iter() {
        let test_data = generate_test_data(*size);

        group.throughput(Throughput::Elements(u64::try_from(*size).unwrap_or(0)));
        group.bench_with_input(BenchmarkId::new("validation_memory", size), size, |b, _| {
            b.iter(|| {
                let schema_ref = Arc::clone(&schema);
                let data = test_data.clone();
                rt.block_on(async move {
                    let engine = Arc::new(require_ok(
                        ValidationEngine::new(schema_ref.as_ref()),
                        "Validation engine creation failed",
                    ));
                    for person in &data {
                        let report = require_ok(
                            engine
                                .validate_as_class(black_box(person), "Person", None)
                                .await,
                            "Validation should succeed",
                        );
                        black_box(report);
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
