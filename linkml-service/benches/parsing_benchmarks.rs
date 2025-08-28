//! Performance benchmarks for LinkML parsing and schema operations

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use linkml_service::SchemaView;
use linkml_service::parser::{JsonParser, YamlParser};
use linkml_service::transform::{InheritanceResolver, SchemaMerger};

const SMALL_YAML_SCHEMA: &str = r#"
id: https://example.org/small
name: small_schema
version: 1.0.0

classes:
  Person:
    slots:
      - name
      - age

slots:
  name:
    range: string
    required: true
  age:
    range: integer
"#;

const MEDIUM_YAML_SCHEMA: &str = r#"
id: https://example.org/medium
name: medium_schema
version: 1.0.0

types:
  PositiveInt:
    base: integer
    minimum_value: 0

enums:
  Status:
    permissible_values:
      - active
      - inactive
      - pending

classes:
  NamedThing:
    abstract: true
    slots:
      - id
      - name
      - description

  Person:
    is_a: NamedThing
    slots:
      - birth_date
      - email
      - status
    slot_usage:
      name:
        required: true

  Organization:
    is_a: NamedThing
    slots:
      - founded_date
      - employees

  Employee:
    is_a: Person
    slots:
      - employee_id
      - department
      - salary

slots:
  id:
    identifier: true
    range: string
  name:
    range: string
  description:
    range: string
  birth_date:
    range: date
  email:
    range: string
    pattern: "^\\S+@\\S+\\.\\S+$"
  status:
    range: Status
  founded_date:
    range: date
  employees:
    range: Employee
    multivalued: true
  employee_id:
    range: string
  department:
    range: string
  salary:
    range: PositiveInt
"#;

fn generate_large_yaml_schema() -> String {
    let mut schema =
        String::from("id: https://example.org/large\nname: large_schema\nversion: 1.0.0\n\n");

    // Add types
    schema.push_str("types:\n");
    for i in 0..20 {
        schema.push_str(&format!(
            "  Type{}:\n    base: string\n    pattern: \"^type{}.*$\"\n",
            i, i
        ));
    }

    // Add enums
    schema.push_str("\nenums:\n");
    for i in 0..10 {
        schema.push_str(&format!("  Enum{}:\n    permissible_values:\n", i));
        for j in 0..10 {
            schema.push_str(&format!("      - value_{}_{}\n", i, j));
        }
    }

    // Add classes
    schema.push_str("\nclasses:\n");
    for i in 0..50 {
        schema.push_str(&format!("  Class{}:\n", i));
        if i > 0 {
            schema.push_str(&format!("    is_a: Class{}\n", i / 2));
        }
        schema.push_str("    slots:\n");
        for j in 0..10 {
            schema.push_str(&format!("      - slot_{}_{}\n", i, j));
        }
    }

    // Add slots
    schema.push_str("\nslots:\n");
    for i in 0..50 {
        for j in 0..10 {
            schema.push_str(&format!(
                "  slot_{}_{}:\n    range: {}\n",
                i,
                j,
                match j % 4 {
                    0 => "string",
                    1 => "integer",
                    2 => format!("Type{}", j % 20),
                    _ => format!("Enum{}", j % 10),
                }
            ));
            if j % 3 == 0 {
                schema.push_str("    required: true\n");
            }
            if j % 5 == 0 {
                schema.push_str("    multivalued: true\n");
            }
        }
    }

    schema
}

fn bench_yaml_parsing(c: &mut Criterion) {
    let parser = YamlParser::new();
    let large_schema = generate_large_yaml_schema();

    let mut group = c.benchmark_group("yaml_parsing");

    group.bench_function("small", |b| {
        b.iter(|| {
            let result = parser.parse(black_box(SMALL_YAML_SCHEMA));
            assert!(result.is_ok());
        })
    });

    group.bench_function("medium", |b| {
        b.iter(|| {
            let result = parser.parse(black_box(MEDIUM_YAML_SCHEMA));
            assert!(result.is_ok());
        })
    });

    group.bench_function("large", |b| {
        b.iter(|| {
            let result = parser.parse(black_box(&large_schema));
            assert!(result.is_ok());
        })
    });

    group.finish();
}

fn bench_json_parsing(c: &mut Criterion) {
    let yaml_parser = YamlParser::new();
    let json_parser = JsonParser::new();

    // Convert YAML schemas to JSON
    let small_schema = yaml_parser.parse(SMALL_YAML_SCHEMA)?;
    let small_json = serde_json::to_string(&small_schema)?;

    let medium_schema = yaml_parser.parse(MEDIUM_YAML_SCHEMA)?;
    let medium_json = serde_json::to_string(&medium_schema)?;

    let large_yaml = generate_large_yaml_schema();
    let large_schema = yaml_parser.parse(&large_yaml)?;
    let large_json = serde_json::to_string(&large_schema)?;

    let mut group = c.benchmark_group("json_parsing");

    group.bench_function("small", |b| {
        b.iter(|| {
            let result = json_parser.parse(black_box(&small_json));
            assert!(result.is_ok());
        })
    });

    group.bench_function("medium", |b| {
        b.iter(|| {
            let result = json_parser.parse(black_box(&medium_json));
            assert!(result.is_ok());
        })
    });

    group.bench_function("large", |b| {
        b.iter(|| {
            let result = json_parser.parse(black_box(&large_json));
            assert!(result.is_ok());
        })
    });

    group.finish();
}

fn bench_inheritance_resolution(c: &mut Criterion) {
    let parser = YamlParser::new();

    let small_schema = parser.parse(SMALL_YAML_SCHEMA)?;
    let medium_schema = parser.parse(MEDIUM_YAML_SCHEMA)?;
    let large_schema = parser.parse(&generate_large_yaml_schema())?;

    let mut group = c.benchmark_group("inheritance_resolution");

    group.bench_function("small", |b| {
        b.iter(|| {
            let mut schema = black_box(small_schema.clone());
            let resolver = InheritanceResolver::new();
            let result = resolver.resolve(&mut schema);
            assert!(result.is_ok());
        })
    });

    group.bench_function("medium", |b| {
        b.iter(|| {
            let mut schema = black_box(medium_schema.clone());
            let resolver = InheritanceResolver::new();
            let result = resolver.resolve(&mut schema);
            assert!(result.is_ok());
        })
    });

    group.bench_function("large", |b| {
        b.iter(|| {
            let mut schema = black_box(large_schema.clone());
            let resolver = InheritanceResolver::new();
            let result = resolver.resolve(&mut schema);
            assert!(result.is_ok());
        })
    });

    group.finish();
}

fn bench_schema_view_operations(c: &mut Criterion) {
    let parser = YamlParser::new();
    let medium_schema = parser.parse(MEDIUM_YAML_SCHEMA)?;
    let large_schema = parser.parse(&generate_large_yaml_schema())?;

    let medium_view = SchemaView::new(medium_schema.clone());
    let large_view = SchemaView::new(large_schema.clone());

    let mut group = c.benchmark_group("schema_view");

    // Induced slots (includes inheritance resolution)
    group.bench_function("induced_slots_medium", |b| {
        b.iter(|| {
            let slots = medium_view.induced_slots(black_box("Employee"));
            assert!(slots.is_some());
            assert!(!slots?.is_empty());
        })
    });

    group.bench_function("induced_slots_large", |b| {
        b.iter(|| {
            let slots = large_view.induced_slots(black_box("Class49"));
            assert!(slots.is_some());
            assert!(!slots?.is_empty());
        })
    });

    // Class ancestors
    group.bench_function("ancestors_medium", |b| {
        b.iter(|| {
            let ancestors = medium_view.class_ancestors(black_box("Employee"));
            assert!(!ancestors.is_empty());
        })
    });

    group.bench_function("ancestors_large", |b| {
        b.iter(|| {
            let ancestors = large_view.class_ancestors(black_box("Class49"));
            assert!(!ancestors.is_empty());
        })
    });

    // Class descendants
    group.bench_function("descendants_medium", |b| {
        b.iter(|| {
            let descendants = medium_view.class_descendants(black_box("NamedThing"));
            assert!(!descendants.is_empty());
        })
    });

    group.bench_function("descendants_large", |b| {
        b.iter(|| {
            let descendants = large_view.class_descendants(black_box("Class0"));
            assert!(!descendants.is_empty());
        })
    });

    // Schema statistics
    group.bench_function("statistics_medium", |b| {
        b.iter(|| {
            let stats = medium_view.schema_statistics();
            assert!(stats.total_elements > 0);
        })
    });

    group.bench_function("statistics_large", |b| {
        b.iter(|| {
            let stats = large_view.schema_statistics();
            assert!(stats.total_elements > 0);
        })
    });

    group.finish();
}

fn bench_schema_merging(c: &mut Criterion) {
    let parser = YamlParser::new();
    let merger = SchemaMerger::new();

    let schema1 = parser.parse(SMALL_YAML_SCHEMA)?;
    let schema2 = parser.parse(MEDIUM_YAML_SCHEMA)?;

    c.bench_function("schema_merge", |b| {
        b.iter(|| {
            let result = merger.merge(black_box(schema1.clone()), black_box(schema2.clone()));
            assert!(result.is_ok());
        })
    });
}

fn bench_parsing_comparison(c: &mut Criterion) {
    let yaml_parser = YamlParser::new();
    let json_parser = JsonParser::new();

    let yaml_content = MEDIUM_YAML_SCHEMA;
    let schema = yaml_parser.parse(yaml_content)?;
    let json_content = serde_json::to_string(&schema)?;

    let mut group = c.benchmark_group("parsing_comparison");

    group.bench_function("yaml", |b| {
        b.iter(|| {
            let result = yaml_parser.parse(black_box(yaml_content));
            assert!(result.is_ok());
        })
    });

    group.bench_function("json", |b| {
        b.iter(|| {
            let result = json_parser.parse(black_box(&json_content));
            assert!(result.is_ok());
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_yaml_parsing,
    bench_json_parsing,
    bench_inheritance_resolution,
    bench_schema_view_operations,
    bench_schema_merging,
    bench_parsing_comparison
);

criterion_main!(benches);
