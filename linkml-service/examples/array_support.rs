//! Example demonstrating array support in LinkML
//!
//! This example shows how to:
//! 1. Define array specifications for scientific data
//! 2. Validate array data
//! 3. Generate code with array support
//! 4. Work with multi-dimensional data

use linkml_core::prelude::*;
use linkml_service::array::{ArrayData, ArrayDimension, ArraySpec, ArrayValidator};
use linkml_service::generator::array_support::{ArrayCodeGenerator, get_array_generator};
use serde_json::json;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("=== LinkML Array Support Example ===
");

    // Example 1: Scientific data arrays
    println!("1. Scientific Data Arrays");
    println!("========================
");

    // Create a 3D array spec for imaging data (e.g., microscopy)
    let imaging_spec = ArraySpec::new("float")
        .with_dimension(ArrayDimension::fixed("z", 50).with_description("Z-axis (depth)"))
        .with_dimension(ArrayDimension::fixed("y", 512).with_description("Y-axis (height)"))
        .with_dimension(ArrayDimension::fixed("x", 512).with_description("X-axis (width)"));

    println!("Imaging data specification:");
    println!(
        "  Dimensions: {:?}",
        imaging_spec
            .dimensions
            .iter()
            .map(|d| format!("{} ({})", d.name, d.size?))
            .collect::<Vec<_>>()
    );
    println!("  Element type: {}", imaging_spec.element_type);
    println!(
        "  Total elements: {}",
        ArraySpec::calculate_size(&[50, 512, 512])
    );
    println!();

    // Example 2: Time series data with dynamic dimension
    println!("2. Time Series Data");
    println!("==================
");

    let timeseries_spec = ArraySpec::new("float")
        .with_dimension(
            ArrayDimension::dynamic("time")
                .with_min(1)
                .with_max(10000)
                .with_description("Time points"),
        )
        .with_dimension(ArrayDimension::fixed("channels", 64).with_description("EEG channels"));

    println!("Time series specification:");
    println!("  Time dimension: dynamic (1-10000 points)");
    println!("  Channels: 64 fixed");
    println!("  Row-major ordering: {}", timeseries_spec.row_major);
    println!();

    // Example 3: Array data manipulation
    println!("3. Array Data Manipulation");
    println!("=========================
");

    // Create a small 2D array for demonstration
    let matrix_spec = ArraySpec::new("integer")
        .with_dimension(ArrayDimension::fixed("rows", 3))
        .with_dimension(ArrayDimension::fixed("cols", 4));

    let data = vec![
        json!(1),
        json!(2),
        json!(3),
        json!(4),
        json!(5),
        json!(6),
        json!(7),
        json!(8),
        json!(9),
        json!(10),
        json!(11),
        json!(12),
    ];

    let matrix = ArrayData::new(matrix_spec.clone(), vec![3, 4], data)?;

    println!("Created 3x4 matrix:");
    for row in 0..3 {
        print!("  [");
        for col in 0..4 {
            print!("{:3}", matrix.get(&[row, col])?);
            if col < 3 {
                print!(", ");
            }
        }
        println!("]");
    }
    println!();

    // Slice operations
    let row_slice = matrix.slice(0, 1)?;
    println!("Row slice at index 1: {:?}", row_slice.data);

    let col_slice = matrix.slice(1, 2)?;
    println!("Column slice at index 2: {:?}", col_slice.data);
    println!();

    // Reshape
    let reshaped = matrix.reshape(vec![2, 6])?;
    println!("Reshaped to 2x6:");
    for row in 0..2 {
        print!("  [");
        for col in 0..6 {
            print!("{:3}", reshaped.get(&[row, col])?);
            if col < 5 {
                print!(", ");
            }
        }
        println!("]");
    }
    println!();

    // Transpose
    let transposed = matrix.transpose();
    println!("Transposed (4x3):");
    for row in 0..4 {
        print!("  [");
        for col in 0..3 {
            print!("{:3}", transposed.get(&[row, col])?);
            if col < 2 {
                print!(", ");
            }
        }
        println!("]");
    }
    println!();

    // Example 4: Code generation with arrays
    println!("4. Code Generation with Arrays");
    println!("=============================
");

    // Generate Python/NumPy code
    if let Some(py_gen) = get_array_generator("python") {
        println!("Python/NumPy code:");
        println!("------------------");

        let type_decl = py_gen.generate_array_type(&imaging_spec, "ImageStack");
        println!("# Type declaration");
        println!("ImageStack = {}", type_decl);
        println!();

        let init_code = py_gen.generate_array_init(&imaging_spec, "image_data");
        println!("# Initialization");
        print!("{}", init_code);
        println!();

        let validation = py_gen.generate_array_validation(&imaging_spec, "image");
        println!("# Validation function");
        print!("{}", validation);
        println!();

        let accessor = py_gen.generate_array_accessor(&imaging_spec, "get_voxel");
        println!("# Accessor methods");
        print!("{}", accessor);
    }

    // Generate TypeScript code
    if let Some(ts_gen) = get_array_generator("typescript") {
        println!("
TypeScript code:");
        println!("----------------");

        let type_decl = ts_gen.generate_array_type(&matrix_spec, "Matrix");
        println!("// Type declaration");
        println!("type Matrix = {};", type_decl);
        println!();

        let init_code = ts_gen.generate_array_init(&matrix_spec, "matrix");
        println!("// Initialization");
        print!("{}", init_code);
        println!();

        let validation = ts_gen.generate_array_validation(&matrix_spec, "matrix");
        println!("// Validation");
        print!("{}", validation);
    }

    // Generate Rust code
    if let Some(rust_gen) = get_array_generator("rust") {
        println!("
Rust code:");
        println!("----------");

        let type_decl = rust_gen.generate_array_type(&timeseries_spec, "TimeSeries");
        println!("// Type declaration");
        println!("type TimeSeries = {};", type_decl);
        println!();

        let init_code = rust_gen.generate_array_init(&timeseries_spec, "data");
        println!("// Initialization");
        print!("{}", init_code);
        println!();

        let validation = rust_gen.generate_array_validation(&timeseries_spec, "TimeSeries");
        println!("// Validation");
        print!("{}", validation);
    }

    // Example 5: Schema integration
    println!("
5. Schema Integration");
    println!("====================
");

    // Create a schema with array slots
    let mut schema = SchemaDefinition::default();
    schema.name = Some("ScientificDataSchema".to_string());

    // Define a class with array data
    let mut measurement_class = ClassDefinition::default();
    measurement_class.description = Some("Scientific measurement with array data".to_string());
    measurement_class.slots = vec![
        "id".to_string(),
        "timestamp".to_string(),
        "sensor_data".to_string(),
        "image_stack".to_string(),
    ];
    schema
        .classes
        .insert("Measurement".to_string(), measurement_class);

    // Define slots
    let mut id_slot = SlotDefinition::default();
    id_slot.identifier = Some(true);
    id_slot.range = Some("string".to_string());
    schema.slots.insert("id".to_string(), id_slot);

    let mut timestamp_slot = SlotDefinition::default();
    timestamp_slot.range = Some("datetime".to_string());
    schema.slots.insert("timestamp".to_string(), timestamp_slot);

    // Array slots would have additional metadata
    let mut sensor_slot = SlotDefinition::default();
    sensor_slot.range = Some("float".to_string());
    sensor_slot.multivalued = Some(true);
    sensor_slot.description = Some("Multi-channel sensor readings".to_string());
    // In a real implementation, we'd attach ArraySpec to the slot
    schema.slots.insert("sensor_data".to_string(), sensor_slot);

    let mut image_slot = SlotDefinition::default();
    image_slot.range = Some("float".to_string());
    image_slot.multivalued = Some(true);
    image_slot.description = Some("3D image stack".to_string());
    schema.slots.insert("image_stack".to_string(), image_slot);

    println!("Created schema with array slots:");
    println!("  - sensor_data: Multi-channel time series");
    println!("  - image_stack: 3D imaging data");
    println!();

    // Example 6: Validation
    println!("6. Array Validation");
    println!("==================
");

    // Create test data
    let test_spec = ArraySpec::new("float")
        .with_dimension(ArrayDimension::fixed("x", 2))
        .with_dimension(ArrayDimension::fixed("y", 2));

    let valid_data = ArrayData::new(
        test_spec.clone(),
        vec![2, 2],
        vec![json!(1.0), json!(2.0), json!(3.0), json!(4.0)],
    )?;

    println!("Validating correct array: ");
    match ArrayValidator::validate(&valid_data, &test_spec) {
        Ok(_) => println!("  ✓ Validation passed"),
        Err(e) => println!("  ✗ Validation failed: {}", e),
    }

    // Try invalid shape
    println!("
Validating incorrect shape:");
    match ArrayData::new(
        test_spec.clone(),
        vec![2, 3], // Wrong shape!
        vec![
            json!(1.0),
            json!(2.0),
            json!(3.0),
            json!(4.0),
            json!(5.0),
            json!(6.0),
        ],
    ) {
        Ok(_) => println!("  ✗ Should have failed"),
        Err(e) => println!("  ✓ Correctly rejected: {}", e),
    }

    // Try wrong type
    let wrong_type_data = ArrayData::new(
        test_spec.clone(),
        vec![2, 2],
        vec![json!("a"), json!("b"), json!("c"), json!("d")], // Strings instead of floats
    )?;

    println!("
Validating incorrect type:");
    match ArrayValidator::validate(&wrong_type_data, &test_spec) {
        Ok(_) => println!("  ✗ Should have failed"),
        Err(e) => println!("  ✓ Correctly rejected: {}", e),
    }

    println!("
✅ Array support examples complete!");
    println!("
Key features demonstrated:");
    println!("- N-dimensional array specifications");
    println!("- Fixed and dynamic dimensions");
    println!("- Array data manipulation (slice, reshape, transpose)");
    println!("- Language-specific code generation");
    println!("- Integration with LinkML schemas");
    println!("- Comprehensive validation");

    Ok(())
}
