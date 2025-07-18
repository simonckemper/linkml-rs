//! Performance benchmarks for LinkML code generation

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use linkml_core::types::{SchemaDefinition, ClassDefinition, SlotDefinition, EnumDefinition, PermissibleValue};
use linkml_service::generator::{
    Generator, GeneratorOptions,
    PythonDataclassGenerator, PydanticGenerator, TypeScriptGenerator,
    JavaScriptGenerator, RustGenerator, JavaGenerator,
    JsonSchemaGenerator, ProtobufGenerator
};

fn create_small_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::new("small_schema");
    schema.id = "https://example.org/small".to_string();
    
    // One class with few fields
    let mut person_class = ClassDefinition::new("Person");
    person_class.slots = vec!["name".to_string(), "age".to_string()];
    schema.classes.insert("Person".to_string(), person_class);
    
    let mut name_slot = SlotDefinition::new("name");
    name_slot.range = Some("string".to_string());
    schema.slots.insert("name".to_string(), name_slot);
    
    let mut age_slot = SlotDefinition::new("age");
    age_slot.range = Some("integer".to_string());
    schema.slots.insert("age".to_string(), age_slot);
    
    schema
}

fn create_medium_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::new("medium_schema");
    schema.id = "https://example.org/medium".to_string();
    
    // Add enum
    let status_enum = EnumDefinition {
        name: "Status".to_string(),
        permissible_values: vec![
            PermissibleValue::Simple("active".to_string()),
            PermissibleValue::Simple("inactive".to_string()),
            PermissibleValue::Simple("pending".to_string()),
        ],
        ..Default::default()
    };
    schema.enums.insert("Status".to_string(), status_enum);
    
    // 10 classes with inheritance
    for i in 0..10 {
        let mut class = ClassDefinition::new(&format!("Entity{}", i));
        if i > 0 {
            class.is_a = Some(format!("Entity{}", i - 1));
        }
        
        // 5 slots per class
        for j in 0..5 {
            let slot_name = format!("field_{}_{}", i, j);
            if !schema.slots.contains_key(&slot_name) {
                let mut slot = SlotDefinition::new(&slot_name);
                slot.range = Some(match j % 3 {
                    0 => "string",
                    1 => "integer",
                    _ => "boolean",
                }.to_string());
                schema.slots.insert(slot_name.clone(), slot);
            }
            class.slots.push(slot_name);
        }
        
        schema.classes.insert(format!("Entity{}", i), class);
    }
    
    schema
}

fn create_large_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::new("large_schema");
    schema.id = "https://example.org/large".to_string();
    
    // Multiple enums
    for i in 0..5 {
        let enum_def = EnumDefinition {
            name: format!("Enum{}", i),
            permissible_values: (0..20)
                .map(|j| PermissibleValue::Simple(format!("VALUE_{}_{}", i, j)))
                .collect(),
            ..Default::default()
        };
        schema.enums.insert(format!("Enum{}", i), enum_def);
    }
    
    // 50 classes with complex relationships
    for i in 0..50 {
        let mut class = ClassDefinition::new(&format!("Class{}", i));
        
        // Inheritance
        if i > 0 && i % 3 == 0 {
            class.is_a = Some(format!("Class{}", i / 3));
        }
        
        // Mixins
        if i % 5 == 0 && i > 4 {
            class.mixins = vec![format!("Class{}", i - 5)];
        }
        
        // 10 slots per class
        for j in 0..10 {
            let slot_name = format!("slot_{}_{}", i, j);
            if !schema.slots.contains_key(&slot_name) {
                let mut slot = SlotDefinition::new(&slot_name);
                slot.range = Some(match j % 5 {
                    0 => "string",
                    1 => "integer",
                    2 => "boolean",
                    3 => format!("Enum{}", j % 5),
                    _ => format!("Class{}", (i + j) % 50),
                }.to_string());
                
                if j % 3 == 0 {
                    slot.required = Some(true);
                }
                if j % 4 == 0 {
                    slot.multivalued = Some(true);
                }
                
                schema.slots.insert(slot_name.clone(), slot);
            }
            class.slots.push(slot_name);
        }
        
        schema.classes.insert(format!("Class{}", i), class);
    }
    
    schema
}

async fn bench_generator<G: Generator>(
    c: &mut Criterion,
    name: &str,
    generator: G,
    schemas: &[(String, SchemaDefinition)]
) {
    let options = GeneratorOptions::default();
    let mut group = c.benchmark_group(name);
    
    for (size_name, schema) in schemas {
        group.bench_with_input(
            BenchmarkId::from_parameter(size_name),
            schema,
            |b, schema| {
                b.iter(|| {
                    let runtime = tokio::runtime::Runtime::new().unwrap();
                    runtime.block_on(async {
                        let result = generator.generate(black_box(schema), &options).await;
                        assert!(result.is_ok());
                        result.unwrap()
                    })
                })
            }
        );
    }
    
    group.finish();
}

fn bench_python_dataclass_generation(c: &mut Criterion) {
    let schemas = vec![
        ("small".to_string(), create_small_schema()),
        ("medium".to_string(), create_medium_schema()),
        ("large".to_string(), create_large_schema()),
    ];
    
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        bench_generator(
            c,
            "python_dataclass",
            PythonDataclassGenerator::new(),
            &schemas
        ).await;
    });
}

fn bench_pydantic_generation(c: &mut Criterion) {
    let schemas = vec![
        ("small".to_string(), create_small_schema()),
        ("medium".to_string(), create_medium_schema()),
        ("large".to_string(), create_large_schema()),
    ];
    
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        bench_generator(
            c,
            "pydantic",
            PydanticGenerator::new(),
            &schemas
        ).await;
    });
}

fn bench_typescript_generation(c: &mut Criterion) {
    let schemas = vec![
        ("small".to_string(), create_small_schema()),
        ("medium".to_string(), create_medium_schema()),
        ("large".to_string(), create_large_schema()),
    ];
    
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        bench_generator(
            c,
            "typescript",
            TypeScriptGenerator::new(),
            &schemas
        ).await;
    });
}

fn bench_rust_generation(c: &mut Criterion) {
    let schemas = vec![
        ("small".to_string(), create_small_schema()),
        ("medium".to_string(), create_medium_schema()),
        ("large".to_string(), create_large_schema()),
    ];
    
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        bench_generator(
            c,
            "rust",
            RustGenerator::new(),
            &schemas
        ).await;
    });
}

fn bench_json_schema_generation(c: &mut Criterion) {
    let schemas = vec![
        ("small".to_string(), create_small_schema()),
        ("medium".to_string(), create_medium_schema()),
        ("large".to_string(), create_large_schema()),
    ];
    
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        bench_generator(
            c,
            "json_schema",
            JsonSchemaGenerator::new(),
            &schemas
        ).await;
    });
}

fn bench_all_generators_comparison(c: &mut Criterion) {
    let schema = create_medium_schema();
    let options = GeneratorOptions::default();
    
    let mut group = c.benchmark_group("generator_comparison");
    
    // Python Dataclass
    group.bench_function("python_dataclass", |b| {
        b.iter(|| {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let generator = PythonDataclassGenerator::new();
                generator.generate(black_box(&schema), &options).await.unwrap()
            })
        })
    });
    
    // Pydantic
    group.bench_function("pydantic", |b| {
        b.iter(|| {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let generator = PydanticGenerator::new();
                generator.generate(black_box(&schema), &options).await.unwrap()
            })
        })
    });
    
    // TypeScript
    group.bench_function("typescript", |b| {
        b.iter(|| {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let generator = TypeScriptGenerator::new();
                generator.generate(black_box(&schema), &options).await.unwrap()
            })
        })
    });
    
    // JavaScript
    group.bench_function("javascript", |b| {
        b.iter(|| {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let generator = JavaScriptGenerator::new();
                generator.generate(black_box(&schema), &options).await.unwrap()
            })
        })
    });
    
    // Rust
    group.bench_function("rust", |b| {
        b.iter(|| {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let generator = RustGenerator::new();
                generator.generate(black_box(&schema), &options).await.unwrap()
            })
        })
    });
    
    // Java
    group.bench_function("java", |b| {
        b.iter(|| {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let generator = JavaGenerator::new();
                generator.generate(black_box(&schema), &options).await.unwrap()
            })
        })
    });
    
    // JSON Schema
    group.bench_function("json_schema", |b| {
        b.iter(|| {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let generator = JsonSchemaGenerator::new();
                generator.generate(black_box(&schema), &options).await.unwrap()
            })
        })
    });
    
    // Protocol Buffers
    group.bench_function("protobuf", |b| {
        b.iter(|| {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let generator = ProtobufGenerator::new();
                generator.generate(black_box(&schema), &options).await.unwrap()
            })
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_python_dataclass_generation,
    bench_pydantic_generation,
    bench_typescript_generation,
    bench_rust_generation,
    bench_json_schema_generation,
    bench_all_generators_comparison
);

criterion_main!(benches);