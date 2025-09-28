//! Performance benchmarks for LinkML parsing and schema operations

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use linkml_service::parser::{JsonParser, SchemaParser, YamlParser};
use linkml_service::schema_view::SchemaView;
use linkml_service::transform::schema_merger::SchemaMerger;
use std::fmt::Display;

fn require_ok<T, E>(result: Result<T, E>, context: &str) -> T
where
    E: Display,
{
    match result {
        Ok(value) => value,
        Err(err) => panic!("{context}: {err}"),
    }
}

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
    let mut schema = String::from(
        "id: https://example.org/large
name: large_schema
version: 1.0.0

",
    );

    // Add types
    schema.push_str(
        "types:
",
    );
    for i in 0..20 {
        schema.push_str(&format!(
            "  Type{}:
    base: string
    pattern: \"^type{}.*$\"
",
            i, i
        ));
    }

    // Add enums
    schema.push_str(
        "
enums:
",
    );
    for i in 0..10 {
        schema.push_str(&format!(
            "  Enum{}:
    permissible_values:
",
            i
        ));
        for j in 0..10 {
            schema.push_str(&format!(
                "      - value_{}_{}
",
                i, j
            ));
        }
    }

    // Add classes
    schema.push_str(
        "
classes:
",
    );
    for i in 0..50 {
        schema.push_str(&format!(
            "  Class{}:
",
            i
        ));
        if i > 0 {
            schema.push_str(&format!(
                "    is_a: Class{}
",
                i / 2
            ));
        }
        schema.push_str(
            "    slots:
",
        );
        for j in 0..10 {
            schema.push_str(&format!(
                "      - slot_{}_{}
",
                i, j
            ));
        }
    }

    // Add slots
    schema.push_str(
        "
slots:
",
    );
    for i in 0..50 {
        for j in 0..10 {
            let range_value = match j % 4 {
                0 => "string".to_string(),
                1 => "integer".to_string(),
                2 => format!("Type{}", j % 20),
                _ => format!("Enum{}", j % 10),
            };
            schema.push_str(&format!(
                "  slot_{}_{}:
    range: {}
",
                i, j, range_value
            ));
            if j % 3 == 0 {
                schema.push_str(
                    "    required: true
",
                );
            }
            if j % 5 == 0 {
                schema.push_str(
                    "    multivalued: true
",
                );
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
    let small_schema = require_ok(
        yaml_parser.parse(SMALL_YAML_SCHEMA),
        "Failed to parse small YAML schema",
    );
    let small_json = require_ok(
        serde_json::to_string(&small_schema),
        "Failed to serialize small schema to JSON",
    );

    let medium_schema = require_ok(
        yaml_parser.parse(MEDIUM_YAML_SCHEMA),
        "Failed to parse medium YAML schema",
    );
    let medium_json = require_ok(
        serde_json::to_string(&medium_schema),
        "Failed to serialize medium schema to JSON",
    );

    let large_yaml = generate_large_yaml_schema();
    let large_schema = require_ok(
        yaml_parser.parse(&large_yaml),
        "Failed to parse generated large YAML schema",
    );
    let large_json = require_ok(
        serde_json::to_string(&large_schema),
        "Failed to serialize large schema to JSON",
    );

    let mut group = c.benchmark_group("json_parsing");

    group.bench_function("small", |b| {
        b.iter(|| {
            let result = json_parser.parse_str(black_box(&small_json));
            assert!(result.is_ok());
        })
    });

    group.bench_function("medium", |b| {
        b.iter(|| {
            let result = json_parser.parse_str(black_box(&medium_json));
            assert!(result.is_ok());
        })
    });

    group.bench_function("large", |b| {
        b.iter(|| {
            let result = json_parser.parse_str(black_box(&large_json));
            assert!(result.is_ok());
        })
    });

    group.finish();
}

fn bench_inheritance_resolution(c: &mut Criterion) {
    let parser = YamlParser::new();

    let small_schema = require_ok(
        parser.parse(SMALL_YAML_SCHEMA),
        "Failed to parse small schema for inheritance resolution",
    );
    let medium_schema = require_ok(
        parser.parse(MEDIUM_YAML_SCHEMA),
        "Failed to parse medium schema for inheritance resolution",
    );
    let large_schema = require_ok(
        parser.parse(&generate_large_yaml_schema()),
        "Failed to parse large schema for inheritance resolution",
    );

    let mut group = c.benchmark_group("inheritance_resolution");

    group.bench_function("small", |b| {
        b.iter(|| {
            let view = require_ok(
                SchemaView::new(small_schema.clone()),
                "SchemaView creation should succeed",
            );
            black_box(view.schema_name().ok());
        })
    });

    group.bench_function("medium", |b| {
        b.iter(|| {
            let view = require_ok(
                SchemaView::new(medium_schema.clone()),
                "SchemaView creation should succeed",
            );
            black_box(view.schema_name().ok());
        })
    });

    group.bench_function("large", |b| {
        b.iter(|| {
            let view = require_ok(
                SchemaView::new(large_schema.clone()),
                "SchemaView creation should succeed",
            );
            black_box(view.schema_name().ok());
        })
    });

    group.finish();
}

fn bench_schema_view_operations(c: &mut Criterion) {
    let parser = YamlParser::new();
    let medium_schema = require_ok(
        parser.parse(MEDIUM_YAML_SCHEMA),
        "Failed to parse medium schema for schema view benchmarks",
    );
    let large_schema = require_ok(
        parser.parse(&generate_large_yaml_schema()),
        "Failed to parse large schema for schema view benchmarks",
    );

    let medium_view = require_ok(
        SchemaView::new(medium_schema.clone()),
        "Failed to construct SchemaView for medium schema",
    );
    let large_view = require_ok(
        SchemaView::new(large_schema.clone()),
        "Failed to construct SchemaView for large schema",
    );

    let mut group = c.benchmark_group("schema_view");

    // Class slot retrieval (includes inheritance resolution)
    group.bench_function("class_slots_medium", |b| {
        b.iter(|| {
            let slots = require_ok(
                medium_view.class_slots(black_box("Employee")),
                "Class slots should resolve for Employee",
            );
            assert!(!slots.is_empty());
        })
    });

    group.bench_function("class_slots_large", |b| {
        b.iter(|| {
            let slots = require_ok(
                large_view.class_slots(black_box("Class49")),
                "Class slots should resolve for Class49",
            );
            assert!(!slots.is_empty());
        })
    });

    // Class ancestors
    group.bench_function("ancestors_medium", |b| {
        b.iter(|| {
            let ancestors = require_ok(
                medium_view.class_ancestors(black_box("Employee")),
                "Ancestor resolution failed",
            );
            assert!(!ancestors.is_empty());
        })
    });

    group.bench_function("ancestors_large", |b| {
        b.iter(|| {
            let ancestors = require_ok(
                large_view.class_ancestors(black_box("Class49")),
                "Ancestor resolution failed",
            );
            assert!(!ancestors.is_empty());
        })
    });

    // Class descendants
    group.bench_function("descendants_medium", |b| {
        b.iter(|| {
            let descendants = require_ok(
                medium_view.class_descendants(black_box("NamedThing")),
                "Descendant resolution failed",
            );
            assert!(!descendants.is_empty());
        })
    });

    group.bench_function("descendants_large", |b| {
        b.iter(|| {
            let descendants = require_ok(
                large_view.class_descendants(black_box("Class0")),
                "Descendant resolution failed",
            );
            assert!(!descendants.is_empty());
        })
    });

    // Schema statistics
    group.bench_function("class_count_medium", |b| {
        b.iter(|| {
            let classes = require_ok(
                medium_view.all_class_names(),
                "Class listing should succeed",
            );
            assert!(!classes.is_empty());
        })
    });

    group.bench_function("class_count_large", |b| {
        b.iter(|| {
            let classes = require_ok(large_view.all_class_names(), "Class listing should succeed");
            assert!(!classes.is_empty());
        })
    });

    group.finish();
}

fn bench_schema_merging(c: &mut Criterion) {
    let parser = YamlParser::new();
    let mut merger = SchemaMerger::with_defaults();

    let schema1 = require_ok(
        parser.parse(SMALL_YAML_SCHEMA),
        "Failed to parse first schema for merge benchmark",
    );
    let schema2 = require_ok(
        parser.parse(MEDIUM_YAML_SCHEMA),
        "Failed to parse second schema for merge benchmark",
    );

    c.bench_function("schema_merge", |b| {
        b.iter(|| {
            let result = merger.merge_two(schema1.clone(), schema2.clone());
            assert!(result.is_ok());
        })
    });
}

fn bench_parsing_comparison(c: &mut Criterion) {
    let yaml_parser = YamlParser::new();
    let json_parser = JsonParser::new();

    let yaml_content = MEDIUM_YAML_SCHEMA;
    let schema = require_ok(
        yaml_parser.parse(yaml_content),
        "Failed to parse medium schema for comparison benchmark",
    );
    let json_content = require_ok(
        serde_json::to_string(&schema),
        "Failed to serialize medium schema to JSON",
    );

    let mut group = c.benchmark_group("parsing_comparison");

    group.bench_function("yaml", |b| {
        b.iter(|| {
            let result = yaml_parser.parse(black_box(yaml_content));
            assert!(result.is_ok());
        })
    });

    group.bench_function("json", |b| {
        b.iter(|| {
            let result = json_parser.parse_str(black_box(&json_content));
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
