#!/usr/bin/env python3
"""
Performance comparison between Rust LinkML and Python LinkML

This script requires Python LinkML to be installed:
    pip install linkml

Run from the linkml-service directory:
    python3 scripts/performance_comparison.py
"""

import time
import json
import subprocess
import sys
import os
from typing import Dict, List, Tuple

try:
    from linkml_runtime.loaders import yaml_loader
    from linkml_runtime.utils.schemaview import SchemaView
    from linkml.validator import validate
except ImportError:
    print("Error: Python LinkML not installed. Please run: pip install linkml linkml-runtime")
    sys.exit(1)

# Test schemas of different sizes
SIMPLE_SCHEMA = """
id: https://example.org/simple
name: SimpleSchema

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
"""

MEDIUM_SCHEMA = """
id: https://example.org/medium
name: MediumSchema

classes:
  Person:
    slots:
      - name
      - age
      - email
      - address

  Address:
    slots:
      - street
      - city
      - state
      - zip_code

  Organization:
    slots:
      - name
      - employees
      - headquarters

slots:
  name:
    range: string
    required: true
  age:
    range: integer
    minimum_value: 0
    maximum_value: 150
  email:
    range: string
    pattern: "^[\\w.-]+@[\\w.-]+\\.\\w+$"
  street:
    range: string
  city:
    range: string
  state:
    range: string
  zip_code:
    range: string
    pattern: "^\\d{5}$"
  employees:
    range: Person
    multivalued: true
  headquarters:
    range: Address
  address:
    range: Address
"""

TEST_DATA_SIMPLE = {
    "name": "John Doe",
    "age": 30
}

TEST_DATA_MEDIUM = {
    "name": "Acme Corp",
    "employees": [
        {"name": "John Doe", "age": 30, "email": "john@acme.com"},
        {"name": "Jane Smith", "age": 25, "email": "jane@acme.com"}
    ],
    "headquarters": {
        "street": "123 Main St",
        "city": "Anytown",
        "state": "CA",
        "zip_code": "12345"
    }
}


def measure_time(func, *args, **kwargs) -> Tuple[float, any]:
    """Measure execution time of a function"""
    start = time.time()
    result = func(*args, **kwargs)
    end = time.time()
    return (end - start) * 1000, result  # Return time in milliseconds


def benchmark_python_linkml(schema_yaml: str, data: Dict, target_class: str) -> Dict[str, float]:
    """Benchmark Python LinkML operations"""
    results = {}

    # Measure parsing time
    parse_time, schema = measure_time(yaml_loader.load, schema_yaml, target_class="SchemaDefinition")
    results['parse_ms'] = parse_time

    # Measure SchemaView creation
    view_time, schema_view = measure_time(SchemaView, schema)
    results['schema_view_ms'] = view_time

    # Measure validation time
    try:
        validate_time, _ = measure_time(validate, data, schema, target_class)
        results['validate_ms'] = validate_time
    except Exception as e:
        print(f"Python validation error: {e}")
        results['validate_ms'] = -1

    return results


def benchmark_rust_linkml(schema_yaml: str, data: Dict, target_class: str) -> Dict[str, float]:
    """Benchmark Rust LinkML operations using the linkml-cli"""
    results = {}

    # Create temporary files
    schema_file = "/tmp/test_schema.yaml"
    data_file = "/tmp/test_data.json"

    with open(schema_file, 'w') as f:
        f.write(schema_yaml)

    with open(data_file, 'w') as f:
        json.dump(data, f)

    # Find the linkml-cli binary
    cli_path = None
    for path in [
        "./target/release/linkml-cli",
        "./target/debug/linkml-cli",
        "../../../target/release/linkml-cli",
        "../../../target/debug/linkml-cli"
    ]:
        if os.path.exists(path):
            cli_path = path
            break

    if not cli_path:
        print("Error: linkml-cli binary not found. Please build the Rust project first.")
        return {'error': 'Binary not found'}

    # Measure validation time (includes parsing)
    start = time.time()
    try:
        result = subprocess.run([
            cli_path, "validate",
            "--schema", schema_file,
            "--data", data_file,
            "--target-class", target_class
        ], capture_output=True, text=True, timeout=10)

        if result.returncode != 0:
            print(f"Rust validation error: {result.stderr}")
            results['validate_ms'] = -1
        else:
            results['validate_ms'] = (time.time() - start) * 1000
    except subprocess.TimeoutExpired:
        results['validate_ms'] = -1

    # Clean up
    os.unlink(schema_file)
    os.unlink(data_file)

    return results


def print_comparison(name: str, python_results: Dict, rust_results: Dict):
    """Print performance comparison results"""
    print(f"\n{name} Performance Comparison")
    print("=" * 50)

    metrics = [
        ('Parse time', 'parse_ms'),
        ('SchemaView creation', 'schema_view_ms'),
        ('Validation time', 'validate_ms')
    ]

    for label, key in metrics:
        py_time = python_results.get(key, -1)
        rs_time = rust_results.get(key, -1)

        if py_time > 0 and rs_time > 0:
            speedup = py_time / rs_time
            print(f"{label:20} Python: {py_time:8.2f}ms  Rust: {rs_time:8.2f}ms  Speedup: {speedup:6.1f}x")
        elif py_time > 0:
            print(f"{label:20} Python: {py_time:8.2f}ms  Rust: N/A")
        elif rs_time > 0:
            print(f"{label:20} Python: N/A              Rust: {rs_time:8.2f}ms")


def main():
    """Run performance comparison"""
    print("LinkML Performance Comparison: Python vs Rust")
    print("=" * 50)

    # Test simple schema
    print("\nRunning simple schema benchmark...")
    py_simple = benchmark_python_linkml(SIMPLE_SCHEMA, TEST_DATA_SIMPLE, "Person")
    rs_simple = benchmark_rust_linkml(SIMPLE_SCHEMA, TEST_DATA_SIMPLE, "Person")
    print_comparison("Simple Schema", py_simple, rs_simple)

    # Test medium schema
    print("\nRunning medium schema benchmark...")
    py_medium = benchmark_python_linkml(MEDIUM_SCHEMA, TEST_DATA_MEDIUM, "Organization")
    rs_medium = benchmark_rust_linkml(MEDIUM_SCHEMA, TEST_DATA_MEDIUM, "Organization")
    print_comparison("Medium Schema", py_medium, rs_medium)

    # Summary
    print("\n" + "=" * 50)
    print("Summary:")
    print("- Rust LinkML typically shows significant performance improvements")
    print("- Actual speedup varies by operation and schema complexity")
    print("- For production workloads, consider running with larger schemas")

    # Note about comprehensive testing
    print("\nNote: For comprehensive benchmarks, run:")
    print("  cargo bench -p linkml-service")


if __name__ == "__main__":
    main()
