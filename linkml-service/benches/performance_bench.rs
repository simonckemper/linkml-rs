use criterion::{black_box, criterion_group, criterion_main, Criterion};
use linkml_service::parser::Parser;
use linkml_service::validator::ValidationEngine;
use serde_json::json;
use tokio::runtime::Runtime;

fn create_test_schema() -> String {
    r"
id: https://example.org/benchmark
name: BenchmarkSchema

classes:
  Person:
    name: Person
    description: A person for benchmarking
    slots:
      - id
      - name
      - email
      - age
      - address

slots:
  id:
    name: id
    range: string
    required: true
  name:
    name: name
    range: string
    required: true
  email:
    name: email
    range: string
    pattern: '^[^@]+@[^@]+\.[^@]+$'
  age:
    name: age
    range: integer
  address:
    name: address
    range: string
".to_string()
}

fn create_test_data() -> serde_json::Value {
    json!({
        "id": "person-001",
        "name": "John Doe",
        "email": "john.doe@example.com",
        "age": 30,
        "address": "123 Main St, Anytown, USA"
    })
}

fn bench_schema_parsing(c: &mut Criterion) {
    let schema_yaml = create_test_schema();
    
    c.bench_function("schema_parsing", |b| {
        b.iter(|| {
            let parser = Parser::new();
            let schema = parser.parse_str(black_box(&schema_yaml), "yaml");
            black_box(schema)
        })
    });
}

fn bench_validation_engine_creation(c: &mut Criterion) {
    let schema_yaml = create_test_schema();
    let parser = Parser::new();
    let schema = parser.parse_str(&schema_yaml, "yaml").unwrap();
    
    c.bench_function("validation_engine_creation", |b| {
        b.iter(|| {
            let engine = ValidationEngine::new(black_box(&schema));
            black_box(engine)
        })
    });
}

fn bench_instance_validation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let schema_yaml = create_test_schema();
    let parser = Parser::new();
    let schema = parser.parse_str(&schema_yaml, "yaml").unwrap();
    let validation_engine = ValidationEngine::new(&schema).unwrap();
    let test_data = create_test_data();
    
    c.bench_function("instance_validation", |b| {
        b.to_async(&rt).iter(|| async {
            let result = validation_engine
                .validate_as_class(black_box(&test_data), "Person", None)
                .await;
            black_box(result)
        })
    });
}

fn bench_concurrent_validation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let schema_yaml = create_test_schema();
    let parser = Parser::new();
    let schema = parser.parse_str(&schema_yaml, "yaml").unwrap();
    let validation_engine = std::sync::Arc::new(ValidationEngine::new(&schema).unwrap());
    let test_data = create_test_data();
    
    c.bench_function("concurrent_validation_10", |b| {
        b.to_async(&rt).iter(|| async {
            let mut handles = vec![];
            
            for _ in 0..10 {
                let engine = validation_engine.clone();
                let data = test_data.clone();
                let handle = tokio::spawn(async move {
                    engine.validate_as_class(&data, "Person", None).await
                });
                handles.push(handle);
            }
            
            let results = futures::future::join_all(handles).await;
            black_box(results)
        })
    });
}

fn bench_large_schema_parsing(c: &mut Criterion) {
    // Create a larger schema with many classes and slots
    let mut large_schema = String::from(r"
id: https://example.org/large-benchmark
name: LargeBenchmarkSchema

classes:
");
    
    // Add 100 classes
    for i in 0..100 {
        large_schema.push_str(&format!(r"
  Class{}:
    name: Class{}
    description: Test class {}
    slots:
      - field{}_1
      - field{}_2
      - field{}_3
", i, i, i, i, i, i));
    }
    
    large_schema.push_str("\nslots:\n");
    
    // Add 300 slots (3 per class)
    for i in 0..100 {
        for j in 1..=3 {
            large_schema.push_str(&format!(r"
  field{}_{j}:
    name: field{}_{j}
    range: string
", i, i));
        }
    }
    
    c.bench_function("large_schema_parsing", |b| {
        b.iter(|| {
            let parser = Parser::new();
            let schema = parser.parse_str(black_box(&large_schema), "yaml");
            black_box(schema)
        })
    });
}

fn bench_memory_usage(c: &mut Criterion) {
    c.bench_function("memory_efficiency", |b| {
        b.iter(|| {
            let mut schemas = Vec::new();
            
            // Create multiple schemas to test memory usage
            for i in 0..10 {
                let schema_yaml = format!(r"
id: https://example.org/memory-test-{}
name: MemoryTest{}

classes:
  TestClass{}:
    name: TestClass{}
    slots:
      - test_field

slots:
  test_field:
    name: test_field
    range: string
", i, i, i, i);
                
                let parser = Parser::new();
                let schema = parser.parse_str(&schema_yaml, "yaml").unwrap();
                schemas.push(schema);
            }
            
            black_box(schemas)
        })
    });
}

criterion_group!(
    benches,
    bench_schema_parsing,
    bench_validation_engine_creation,
    bench_instance_validation,
    bench_concurrent_validation,
    bench_large_schema_parsing,
    bench_memory_usage
);
criterion_main!(benches);
